pub mod signal;

#[no_mangle]
pub extern "C" fn start_measurement() -> i32 {
    signal::start_measurement()
}

#[no_mangle]
pub extern "C" fn end_measurement() {
    signal::end_measurement()
}

#[cfg(target_os = "linux")]
#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
pub mod jni {
    use jni::objects::JClass;
    use jni::sys::jint;
    use jni::JNIEnv;

    #[no_mangle]
    pub extern "system" fn Java_Measurements_startMeasurement(
        _env: JNIEnv,
        _class: JClass,
    ) -> jint {
        crate::signal::start_measurement()
    }

    #[no_mangle]
    pub extern "system" fn Java_Measurements_endMeasurement(_env: JNIEnv, _class: JClass) {
        crate::signal::end_measurement();
    }
}
