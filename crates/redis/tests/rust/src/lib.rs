use spin_redis_trigger_v01::*;

wai_bindgen_rust::export!("../../wit/spin_redis_trigger_v01.wit");

struct SpinRedisTriggerV01 {}

impl spin_redis_trigger_v01::SpinRedisTriggerV01 for SpinRedisTriggerV01 {
    fn handler(payload: Payload) {
        let msg = std::str::from_utf8(&payload).expect("cannot read message string from payload");
        println!("Message: {}", msg);
    }
}
