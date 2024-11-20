use wasm_bindgen::prelude::*;
use web_sys::{Element, Window};
use yew::prelude::*;
use js_sys::Function;
use gloo_timers::callback::Interval;

pub enum Msg {
    CaptchaLoaded,
    CaptchaVerified(String),
    CaptchaExpired,
    CaptchaError,
    Reset,
}

#[derive(Properties, PartialEq)]
pub struct Props {
    pub on_verify: Callback<String>,
    pub on_expire: Callback<()>,
    pub on_error: Callback<()>,
    #[prop_or_default]
    pub site_key: String,
    #[prop_or_default]
    pub should_reset: bool,
}

pub struct HCaptcha {
    node_ref: NodeRef,
    _verify_callback: Function,
    _expire_callback: Function,
    _error_callback: Function,
    _interval: Option<Interval>,
}

impl Component for HCaptcha {
    type Message = Msg;
    type Properties = Props;

    fn create(ctx: &Context<Self>) -> Self {
        let on_verify = ctx.props().on_verify.clone();
        let verify_callback = Closure::wrap(Box::new(move |response: JsValue| {
            if let Some(token) = response.as_string() {
                on_verify.emit(token);
            }
        }) as Box<dyn FnMut(JsValue)>).into_js_value();

        let on_expire = ctx.props().on_expire.clone();
        let expire_callback = Closure::wrap(Box::new(move || {
            on_expire.emit(());
        }) as Box<dyn FnMut()>).into_js_value();

        let on_error = ctx.props().on_error.clone();
        let error_callback = Closure::wrap(Box::new(move || {
            on_error.emit(());
        }) as Box<dyn FnMut()>).into_js_value();

        Self {
            node_ref: NodeRef::default(),
            _verify_callback: verify_callback.into(),
            _expire_callback: expire_callback.into(),
            _error_callback: error_callback.into(),
            _interval: None,
        }
    }

    fn changed(&mut self, ctx: &Context<Self>, _old_props: &Self::Properties) -> bool {
        if ctx.props().should_reset {
            self.reset_captcha();
        }
        true
    }

    fn update(&mut self, _ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::Reset => {
                self.reset_captcha();
                true
            }
            _ => false
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        html! {
            <div 
                ref={self.node_ref.clone()} 
                class="h-captcha"
                data-sitekey={ctx.props().site_key.clone()}
                data-callback="onVerify"
                data-expired-callback="onExpire"
                data-error-callback="onError">
            </div>
        }
    }

    fn rendered(&mut self, _ctx: &Context<Self>, first_render: bool) {
        if first_render {
            let node_ref = self.node_ref.clone();
            let verify_callback = self._verify_callback.clone();
            let expire_callback = self._expire_callback.clone();
            let error_callback = self._error_callback.clone();

            let interval = Interval::new(100, move || {
                if let Some(window) = web_sys::window() {
                    let w: &Window = &window;

                    if window.document()
                        .and_then(|doc| doc.get_element_by_id("hcaptcha-script"))
                        .is_none() 
                    {
                        let document = window.document().unwrap();
                        let script: Element = document.create_element("script").unwrap();
                        script.set_id("hcaptcha-script");
                        script.set_attribute("src", "https://js.hcaptcha.com/1/api.js").unwrap();
                        script.set_attribute("async", "true").unwrap();
                        script.set_attribute("defer", "true").unwrap();
                        document.head().unwrap().append_child(&script).unwrap();
                    }

                    if js_sys::Reflect::get(w, &JsValue::from_str("hcaptcha")).is_ok() {
                        js_sys::Reflect::set(
                            w,
                            &JsValue::from_str("onVerify"),
                            &verify_callback,
                        ).expect("Failed to set verify callback");
                        
                        js_sys::Reflect::set(
                            w,
                            &JsValue::from_str("onExpire"),
                            &expire_callback,
                        ).expect("Failed to set expire callback");
                        
                        js_sys::Reflect::set(
                            w,
                            &JsValue::from_str("onError"),
                            &error_callback,
                        ).expect("Failed to set error callback");

                        if let Ok(hcaptcha) = js_sys::Reflect::get(w, &JsValue::from_str("hcaptcha")) {
                            let _ = js_sys::Reflect::get(&hcaptcha, &JsValue::from_str("render"))
                                .and_then(|render| {
                                    if render.is_function() {
                                        let func = js_sys::Function::from(render);
                                        if let Some(element) = node_ref.cast::<Element>() {
                                            func.call1(&hcaptcha, &element.into())
                                        } else {
                                            Ok(JsValue::UNDEFINED)
                                        }
                                    } else {
                                        Ok(JsValue::UNDEFINED)
                                    }
                                });
                        }
                    }
                }
            });

            self._interval = Some(interval);
        }
    }

    fn destroy(&mut self, _ctx: &Context<Self>) {
        self._interval = None;
        if let Some(window) = web_sys::window() {
            let _ = js_sys::Reflect::set(
                &window,
                &JsValue::from_str("onVerify"),
                &JsValue::UNDEFINED,
            );
            let _ = js_sys::Reflect::set(
                &window,
                &JsValue::from_str("onExpire"),
                &JsValue::UNDEFINED,
            );
            let _ = js_sys::Reflect::set(
                &window,
                &JsValue::from_str("onError"),
                &JsValue::UNDEFINED,
            );
        }
    }
}

impl HCaptcha {
    fn reset_captcha(&self) {
        if let Some(window) = web_sys::window() {
            let _ = js_sys::Reflect::get(&window, &JsValue::from_str("hcaptcha"))
                .and_then(|hcaptcha| {
                    js_sys::Reflect::get(&hcaptcha, &JsValue::from_str("reset"))
                        .and_then(|reset| {
                            if reset.is_function() {
                                let func = js_sys::Function::from(reset);
                                func.call0(&hcaptcha)
                            } else {
                                Ok(JsValue::UNDEFINED)
                            }
                        })
                });
        }
    }
}

#[wasm_bindgen]
pub fn reset_hcaptcha() {
    if let Some(window) = web_sys::window() {
        let _ = js_sys::Reflect::get(&window, &JsValue::from_str("hcaptcha"))
            .and_then(|hcaptcha| {
                js_sys::Reflect::get(&hcaptcha, &JsValue::from_str("reset"))
                    .and_then(|reset| {
                        if reset.is_function() {
                            let func = js_sys::Function::from(reset);
                            func.call0(&hcaptcha)
                        } else {
                            Ok(JsValue::UNDEFINED)
                        }
                    })
            });
    }
}