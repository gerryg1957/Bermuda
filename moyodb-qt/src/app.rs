#[cxx_qt::bridge]
mod ffi {
    unsafe extern "RustQt" {
        #[qobject]
        #[qml_element]
        type MoyoDbApp = super::MoyoDbAppRust;
    }
}

#[derive(Default)]
pub struct MoyoDbAppRust;
