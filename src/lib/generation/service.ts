import OpenAI from "openai";

import { ApiError, type Candidate, type GenerateResult, type ProjectDetail } from "@/lib/api/types";
import {
  mapCandidate,
  mapProject,
  toApiError,
  type CandidateDto,
  type GeneratePayloadDto,
} from "@/lib/api/tauri-api";
import {
  createImageClient,
  type ImageClient,
  type InvokeFn,
} from "@/lib/generation/client";

/**
 * Generation orchestration for the cms-direct path (C2-FE): one upstream SDK
 * call (`images.generate` / `images.edit`), then a per-item split of the
 * dual-form response — `b64_json → imageBase64`, `url → imageUrl` — with
 * every image persisted through `record_generated_image` and the returned
 * payload refreshing the canvas.
 *
 * The presigned S3 URL is NEVER fetched here (S3 sends no CORS headers to
 * the webview); it crosses the bridge verbatim and Rust downloads the bytes.
 */

export type GenKind = "board" | "page" | "component";
export type GenEndpoint = "generations" | "edits";

/** One upstream batch the webview itself produced. */
export interface ImageBatchParams {
  projectId: string;
  kind: GenKind;
  /** Page slug / component name; empty for board. */
  target: string;
  endpoint: GenEndpoint;
  /** The exact composed prompt (also the lineage record's prompt). */
  prompt: string;
  count: number;
  /** `"1536x1024"` form. */
  size: string;
  quality: "low" | "medium" | "high";
  /** Reference images for `edits` — anchor first. Empty for generations. */
  references: File[];
  /** Override model; default is the desktop-resolved default model. */
  model?: string;
  signal?: AbortSignal;
}

export interface SdkDeps {
  invokeFn: InvokeFn;
  toUrl: (path: string) => string;
  /** Test seam; defaults to the real SDK client factory. */
  clientFactory?: (invokeFn: InvokeFn) => Promise<ImageClient>;
}

type ImagesResult = Awaited<ReturnType<OpenAI["images"]["generate"]>>;

interface ImageItem {
  b64_json?: string | null;
  url?: string | null;
}

/** `record_generated_image` input (commands.rs `RecordImageInput`). */
export interface RecordImageInput {
  projectId: string;
  kind: GenKind;
  target: string;
  imageBase64?: string;
  imageUrl?: string;
  endpoint: GenEndpoint;
  prompt: string;
  size: string;
  quality: string;
  model: string;
}

/**
 * Map an SDK failure into the typed ApiError vocabulary. 402/quota shapes
 * become the dedicated `QUOTA_EXCEEDED` code (recharge/admin copy); rejected
 * keys map back to sign-in guidance; everything else stays `API_ERROR`.
 * Gateway message text is forwarded (it never carries credentials).
 */
export function mapSdkError(error: unknown): ApiError {
  if (error instanceof ApiError) return error;
  if (error instanceof OpenAI.APIError) {
    const status = typeof error.status === "number" ? error.status : 0;
    const body = (error.error ?? {}) as {
      code?: unknown;
      type?: unknown;
      message?: unknown;
    };
    const code = typeof body.code === "string" ? body.code : typeof body.type === "string" ? body.type : "";
    const message =
      typeof body.message === "string" && body.message.length > 0 ? body.message : error.message;
    if (status === 402 || code.includes("quota")) {
      // The hint marker makes the toast render the localized
      // errors.QUOTA_EXCEEDED.hint (recharge/admin guidance); the raw
      // marker itself is only a fallback and never shown.
      return new ApiError("QUOTA_EXCEEDED", message, "top up or contact the administrator");
    }
    if (status === 401 || status === 403) {
      return new ApiError("NO_CREDENTIALS", message, "the stored key was rejected — sign in again");
    }
    if (error instanceof OpenAI.APIConnectionError) {
      return new ApiError("API_ERROR", message, "the gateway is unreachable — check the network");
    }
    return new ApiError("API_ERROR", message);
  }
  return toApiError(error);
}

/**
 * Turn a canvas asset URL (anchor PNG under ~/Rudder) into an uploadable
 * File for `images.edit`. Local asset protocol only — never a presigned URL.
 */
export async function fetchReferenceFile(url: string, name: string): Promise<File> {
  const response = await fetch(url);
  if (!response.ok) {
    throw new ApiError("API_ERROR", `reference image unreadable: ${name}`);
  }
  const blob = await response.blob();
  return new File([blob], name, { type: blob.type || "image/png" });
}

function mapPayload(toUrl: (path: string) => string, dto: GeneratePayloadDto): {
  candidates: Candidate[];
  project: ProjectDetail;
} {
  return {
    candidates: dto.candidates.map((candidate: CandidateDto) => mapCandidate(toUrl, candidate)),
    project: mapProject(toUrl, dto.project),
  };
}

/**
 * Run one upstream batch and persist every image it produced. Resolves with
 * the union of recorded candidates plus the freshest project payload (the
 * canvas refresh source). All failures arrive as `ApiError`.
 */
export async function runImageBatch(
  params: ImageBatchParams,
  deps: SdkDeps,
): Promise<GenerateResult> {
  const { invokeFn, toUrl } = deps;
  try {
    const imageClient = await (deps.clientFactory ?? createImageClient)(invokeFn);
    const model = params.model ?? imageClient.config.model;
    const requestOptions = { signal: params.signal };

    let images: ImagesResult;
    if (params.endpoint === "generations") {
      images = await imageClient.client.images.generate(
        { model, prompt: params.prompt, n: params.count, size: params.size, quality: params.quality },
        requestOptions,
      );
    } else {
      if (params.references.length === 0) {
        throw new ApiError("ANCHOR_REQUIRED", "edits need at least the board anchor");
      }
      images = await imageClient.client.images.edit(
        {
          model,
          prompt: params.prompt,
          image: params.references,
          n: params.count,
          size: params.size,
          quality: params.quality,
        },
        requestOptions,
      );
    }

    const items = (images.data ?? []) as ImageItem[];
    if (items.length === 0) {
      throw new ApiError("API_ERROR", "the gateway returned no images");
    }

    const candidates: Candidate[] = [];
    let project: ProjectDetail | null = null;
    for (const item of items) {
      const input: RecordImageInput = {
        projectId: params.projectId,
        kind: params.kind,
        target: params.target,
        endpoint: params.endpoint,
        prompt: params.prompt,
        size: params.size,
        quality: params.quality,
        model,
      };
      if (item.b64_json) {
        input.imageBase64 = item.b64_json;
      } else if (item.url) {
        // Presigned URL: cross the bridge verbatim; Rust downloads (no CORS).
        input.imageUrl = item.url;
      } else {
        throw new ApiError("API_ERROR", "an image item had neither b64_json nor url");
      }
      const payload = await invokeFn<GeneratePayloadDto>("record_generated_image", { input });
      const mapped = mapPayload(toUrl, payload);
      candidates.push(...mapped.candidates);
      project = mapped.project;
    }
    return { candidates, project: project as ProjectDetail };
  } catch (error) {
    throw mapSdkError(error);
  }
}
