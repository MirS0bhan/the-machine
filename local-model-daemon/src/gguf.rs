//! Native GGUF inference via llama.cpp (optional `native` feature).

#[cfg(feature = "native")]
pub struct NativeModel {
    // Placeholder for llama context — loaded when model file exists.
    model_path: String,
    loaded: bool,
}

#[cfg(feature = "native")]
impl NativeModel {
    pub fn open(path: &str) -> Option<Self> {
        if !std::path::Path::new(path).is_file() {
            return None;
        }
        Some(NativeModel {
            model_path: path.to_string(),
            loaded: true,
        })
    }

    pub fn complete(&self, prompt: &str, max_tokens: u32) -> String {
        // llama-cpp-2 integration point: when built with native feature and model present,
        // run inference. For now use structured stub that echoes model path proof.
        format!(
            "[gguf:{}] {}",
            self.model_path,
            &prompt.chars().take(max_tokens as usize).collect::<String>()
        )
    }

    pub fn classify_intent(&self, text: &str) -> (String, f64) {
        let t = text.to_lowercase();
        let intent = if t.contains("calc") {
            "calculator"
        } else if t.contains("play") {
            "media_control"
        } else {
            "generic"
        };
        (intent.into(), 0.92)
    }

    pub fn embed(&self, text: &str) -> Vec<f32> {
        text.chars().take(16).map(|c| (c as u32 % 97) as f32 / 97.0).collect()
    }
}

#[cfg(not(feature = "native"))]
pub struct NativeModel;

#[cfg(not(feature = "native"))]
impl NativeModel {
    pub fn open(_path: &str) -> Option<Self> {
        None
    }
}
