use core::time::Duration;
use wasm_bindgen::{prelude::*, JsCast, JsValue};

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_name = "setTimeout", catch)]
    pub fn set_timeout(handler: &js_sys::Function, timeout: i32) -> Result<JsValue, JsValue>;

    # [wasm_bindgen (js_name = clearTimeout)]
    pub fn clear_timeout(handler: JsValue) -> JsValue;
}

pub async fn sleep(dur: Duration) {
    let mut cb = |succ: js_sys::Function, _: js_sys::Function| {
        set_timeout(&succ, (dur.as_secs_f64() * 1000.0) as i32).unwrap_throw();
    };
    let promise = js_sys::Promise::new(&mut cb);
    wasm_bindgen_futures::JsFuture::from(promise)
        .await
        .expect("wasm sleeping failed");
}

pub struct WasmTimeOut {
    inner: Option<Closure<dyn FnMut()>>,
    millis: u32,
    id: Option<JsValue>,
}

impl WasmTimeOut {
    pub fn new<F>(millis: u32, f: F) -> Self
    where
        F: 'static + FnMut(),
    {
        let closure = Closure::<dyn FnMut()>::new(f);
        Self {
            inner: Some(closure),
            millis,
            id: None,
        }
    }

    pub fn execute(&mut self) {
        let id = set_timeout(
            self.inner
                .as_ref()
                .unwrap()
                .as_ref()
                .unchecked_ref::<js_sys::Function>(),
            self.millis as i32,
        )
        .unwrap_throw();
        self.id = Some(id);
    }
}

impl Drop for WasmTimeOut {
    fn drop(&mut self) {
        if let Some(id) = self.id.take() {
            clear_timeout(id);
        }
    }
}
