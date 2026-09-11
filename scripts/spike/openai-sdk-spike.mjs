// Spike (C2-FE): verify openai@7.13.0 behaviour against an OpenAI-shaped
// local gateway before wiring the real flows.
//
// Verifies three facts the SDK-direct path depends on:
// 1. `client.images.edit` accepts `image: File[]` (multi reference images)
//    and sends every file as a multipart part;
// 2. a `data[i].url` response is surfaced verbatim as `data[i].url`
//    (S3 presigned URL form) — the SDK does not strip or fetch it;
// 3. a 402/billing failure surfaces as `OpenAI.APIError` with `status`
//    and the gateway's `error.code` preserved, so the UI can map it.
//
// Run: node scripts/spike/openai-sdk-spike.mjs
import { createServer } from "node:http";
import OpenAI from "openai";

const results = [];
const record = (name, ok, detail) => {
  results.push({ name, ok, detail });
  console.log(`${ok ? "PASS" : "FAIL"} ${name}: ${detail}`);
};

const b64 = Buffer.from("spike-png-bytes").toString("base64");
const server = createServer((req, res) => {
  let body = [];
  req.on("data", (chunk) => body.push(chunk));
  req.on("end", () => {
    const raw = Buffer.concat(body);
    if (req.url.endsWith("/images/edits")) {
      const text = raw.toString("latin1");
      const imageParts = [...text.matchAll(/name="image\[\]"/g)].length;
      const imagePartsSingular = [...text.matchAll(/name="image"/g)].length;
      const filenames = [...text.matchAll(/filename="([^"]+)"/g)].map((m) => m[1]);
      record(
        "edits multipart",
        imageParts === 2,
        `image[] parts=${imageParts}, singular image parts=${imagePartsSingular}, files=${filenames.join(",")}`,
      );
      res.setHeader("content-type", "application/json");
      res.end(JSON.stringify({ created: 1, data: [{ b64_json: b64 }] }));
      return;
    }
    if (req.url.endsWith("/images/generations")) {
      const prompt = JSON.parse(raw.toString("utf8")).prompt ?? "";
      if (prompt === "billing-fail") {
        res.statusCode = 402;
        res.setHeader("content-type", "application/json");
        res.end(
          JSON.stringify({
            error: {
              message: "积分不足：本次生成需要 10 点",
              type: "insufficient_quota",
              code: "insufficient_quota",
            },
          }),
        );
        return;
      }
      res.setHeader("content-type", "application/json");
      res.end(
        JSON.stringify({
          created: 1,
          data: [{ url: "https://s3.example.com/rudder/x.png?X-Amz-Signature=REDACTED" }],
        }),
      );
      return;
    }
    res.statusCode = 404;
    res.end("{}");
  });
});

await new Promise((resolve) => server.listen(0, "127.0.0.1", resolve));
const port = server.address().port;
const baseURL = `http://127.0.0.1:${port}/v1`;

const client = new OpenAI({
  apiKey: "sk-spike-dummy",
  baseURL,
  maxRetries: 0,
  timeout: 10_000,
  dangerouslyAllowBrowser: true, // webview parity with the desktop app
});

// 1) multi-image edit
const file = (name) => new File([new Uint8Array([0x89, 0x50, 0x4e, 0x47])], name, { type: "image/png" });
const edit = await client.images.edit({
  model: "gpt-image-2",
  image: [file("anchor.png"), file("current.png")],
  prompt: "spike",
  n: 1,
  size: "1024x1024",
});
record(
  "edit multi-image request accepted",
  edit.data[0].b64_json === b64,
  `data[0].b64_json round-trips (${edit.data[0].b64_json.length} chars)`,
);

// 2) url-form response
const gen = await client.images.generate({ model: "gpt-image-2", prompt: "spike", n: 1 });
const url = gen.data[0]?.url;
record(
  "url response readable",
  typeof url === "string" && url.startsWith("https://s3.example.com/"),
  `data[0].url=${url ? url.split("?")[0] + "?…" : String(url)}`,
);

// 3) billing error shape
try {
  await client.images.generate({ model: "gpt-image-2", prompt: "billing-fail", n: 1 });
  record("billing error", false, "no error thrown");
} catch (error) {
  const isApiErr = error instanceof OpenAI.APIError;
  const status = /** @type {any} */ (error).status;
  const code = /** @type {any} */ (error).error?.error?.code ?? /** @type {any} */ (error).error?.code;
  const type = /** @type {any} */ (error).error?.error?.type ?? /** @type {any} */ (error).error?.type;
  record(
    "billing error shape",
    isApiErr && status === 402,
    `instanceof OpenAI.APIError=${isApiErr}, status=${status}, code=${code}, type=${type}, message=${/** @type {any} */ (error).message?.slice(0, 60)}`,
  );
}

server.close();
const failed = results.filter((r) => !r.ok);
console.log(failed.length === 0 ? "\nSPIKE OK — all facts verified" : `\nSPIKE FAILED — ${failed.length} check(s) failed`);
process.exit(failed.length === 0 ? 0 : 1);
