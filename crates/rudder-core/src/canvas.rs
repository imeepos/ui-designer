//! Canvas size model and gpt-image-2 constraint validation (ARCHITECTURE §3).
//!
//! Constraints (docs/PRD.md §3.1): each side a multiple of 16, aspect ratio
//! ≤ 3:1 (either orientation), total pixels within 0.65–8.3 megapixels.
//! Presets: `web` 1536x1024 · `mobile` 1024x1536 · `desktop` 2560x1440.

use crate::error::{Result, RudderError};
use serde::{Deserialize, Serialize};

/// Lower bound of the accepted megapixel window (650_000 px).
pub const MIN_PIXELS: u64 = 650_000;
/// Upper bound of the accepted megapixel window (8_300_000 px).
pub const MAX_PIXELS: u64 = 8_300_000;
/// Maximum aspect ratio (long side / short side), inclusive.
pub const MAX_RATIO: f64 = 3.0;

/// Named canvas presets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Preset {
    Web,
    Mobile,
    Desktop,
}

impl Preset {
    /// `(w, h)` for the preset, per docs/PRD.md §3.1.
    pub fn dimensions(self) -> (u32, u32) {
        match self {
            Preset::Web => (1536, 1024),
            Preset::Mobile => (1024, 1536),
            Preset::Desktop => (2560, 1440),
        }
    }

    /// Canonical string form (`web` / `mobile` / `desktop`).
    pub fn as_str(self) -> &'static str {
        match self {
            Preset::Web => "web",
            Preset::Mobile => "mobile",
            Preset::Desktop => "desktop",
        }
    }

    fn parse(s: &str) -> Option<Preset> {
        match s {
            "web" => Some(Preset::Web),
            "mobile" => Some(Preset::Mobile),
            "desktop" => Some(Preset::Desktop),
            _ => None,
        }
    }
}

/// A concrete canvas size, optionally tied to a named preset.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CanvasSize {
    pub w: u32,
    pub h: u32,
    /// `Some(preset)` when the size came from `web|mobile|desktop`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preset: Option<Preset>,
}

impl CanvasSize {
    pub fn new(w: u32, h: u32, preset: Option<Preset>) -> CanvasSize {
        CanvasSize { w, h, preset }
    }

    /// `"1536x1024"` — the form the image API expects.
    pub fn to_api_string(&self) -> String {
        format!("{}x{}", self.w, self.h)
    }

    /// Total pixels.
    pub fn pixels(&self) -> u64 {
        u64::from(self.w) * u64::from(self.h)
    }

    /// Parse a `--size` value: a preset name or custom `WxH` (validated).
    pub fn parse(spec: &str) -> Result<CanvasSize> {
        let spec = spec.trim();
        if let Some(preset) = Preset::parse(spec) {
            let (w, h) = preset.dimensions();
            return Ok(CanvasSize::new(w, h, Some(preset)));
        }
        let detail = || RudderError::SizeInvalid {
            detail: format!(
                "`{spec}` is not a preset (web|mobile|desktop) nor a WxH pair"
            ),
        };
        let (w_raw, h_raw) = spec.split_once('x').or_else(|| spec.split_once('X')).ok_or_else(detail)?;
        let w: u32 = w_raw.trim().parse().map_err(|_| detail())?;
        let h: u32 = h_raw.trim().parse().map_err(|_| detail())?;
        let canvas = CanvasSize::new(w, h, None);
        canvas.validate()?;
        Ok(canvas)
    }

    /// Validate against the gpt-image-2 constraints. Preset sizes are
    /// validated too, so a constraint change cannot pass silently.
    pub fn validate(&self) -> Result<()> {
        let mut problems: Vec<String> = Vec::new();
        if self.w == 0 || self.h == 0 {
            problems.push("width and height must be positive".into());
        }
        if !self.w.is_multiple_of(16) {
            problems.push(format!("width {} is not a multiple of 16", self.w));
        }
        if !self.h.is_multiple_of(16) {
            problems.push(format!("height {} is not a multiple of 16", self.h));
        }
        if self.w > 0 && self.h > 0 {
            let long = self.w.max(self.h) as f64;
            let short = self.w.min(self.h) as f64;
            let ratio = long / short;
            if ratio > MAX_RATIO + f64::EPSILON {
                problems.push(format!("aspect ratio {ratio:.2}:1 exceeds the 3:1 limit"));
            }
        }
        let px = self.pixels();
        if px < MIN_PIXELS {
            problems.push(format!(
                "{} px is below the 0.65 MP minimum ({MIN_PIXELS} px)",
                px
            ));
        }
        if px > MAX_PIXELS {
            problems.push(format!(
                "{} px is above the 8.3 MP maximum ({MAX_PIXELS} px)",
                px
            ));
        }
        if problems.is_empty() {
            Ok(())
        } else {
            Err(RudderError::SizeInvalid {
                detail: problems.join("; "),
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(spec: &str) -> Result<CanvasSize> {
        CanvasSize::parse(spec)
    }

    #[test]
    fn presets_are_valid_and_exact() {
        for (name, (w, h)) in [
            ("web", (1536, 1024)),
            ("mobile", (1024, 1536)),
            ("desktop", (2560, 1440)),
        ] {
            let c = parse(name).unwrap();
            assert_eq!((c.w, c.h), (w, h));
            assert_eq!(c.preset, Some(Preset::parse(name).unwrap()));
            assert!(c.validate().is_ok(), "{name} preset must satisfy constraints");
            assert_eq!(c.to_api_string(), format!("{w}x{h}"));
        }
    }

    #[test]
    fn custom_ok() {
        let c = parse("1024x640").unwrap(); // 655_360 px, ratio 1.6
        assert_eq!((c.w, c.h), (1024, 640));
        assert!(c.preset.is_none());
    }

    #[test]
    fn not_multiple_of_16_rejected() {
        let err = parse("1000x1024").unwrap_err();
        assert_eq!(err.code(), "SIZE_INVALID");
        assert!(err.to_string().contains("multiple of 16"));
        assert!(parse("1024x1000").is_err());
    }

    #[test]
    fn ratio_over_three_rejected() {
        // 4096x1024 = 4:1 (pixels 4.2M would otherwise be fine).
        let err = parse("4096x1024").unwrap_err();
        assert!(err.to_string().contains("3:1"), "{err}");
        // Either orientation counts.
        assert!(parse("1024x4096").is_err());
        // Exactly 3:1 (3072x1024, 3.1M px) is allowed.
        assert!(parse("3072x1024").is_ok());
        assert!(parse("1024x3072").is_ok());
    }

    #[test]
    fn megapixel_window_enforced() {
        // 1024x624 = 638_976 px < 0.65M.
        let err = parse("1024x624").unwrap_err();
        assert!(err.to_string().contains("below"), "{err}");
        // 2560x3264 = 8_355_840 px > 8.3M.
        let err = parse("2560x3264").unwrap_err();
        assert!(err.to_string().contains("above"), "{err}");
        // Exact bounds are unreachable with 16-multiples? 816x800 = 652_800 ok.
        assert!(parse("800x816").is_ok());
        // 2560x3248 = 8_314_880 > 8.3M; 2544x3248 = 8_262_912 ok.
        assert!(parse("2544x3248").is_ok());
    }

    #[test]
    fn malformed_specs_rejected() {
        for bad in ["", "webbig", "abc", "1024", "1024x", "x1024", "10.24x640", "1024x0", "0x0"] {
            let err = parse(bad).unwrap_err();
            assert_eq!(err.code(), "SIZE_INVALID", "spec {bad:?}");
            assert_eq!(err.exit_code(), 1);
        }
    }

    #[test]
    fn uppercase_x_accepted() {
        assert_eq!(parse("1024X640").unwrap(), CanvasSize::new(1024, 640, None));
    }
}
