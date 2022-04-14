package outbound_redis

// #include "outbound-redis.h"
import "C"
import (
	"errors"
	"unsafe"
)

func publish(addr, channel string, payload []byte) error {
	caddr := redisStr(addr)
	cchannel := redisStr(channel)
	cpayload := C.outbound_redis_payload_t{ptr: &payload[0], len: C.size_t(len(payload))}

	defer func() {
		C.outbound_redis_string_free(&caddr)
		C.outbound_redis_string_free(&cchannel)
		C.outbound_redis_payload_free(&cpayload)
	}()

	err := C.outbound_redis_publish(&caddr, &cchannel, &cpayload)
	return toErr(err)
}

func get(addr, key string) ([]byte, error) {
	caddr := redisStr(addr)
	ckey := redisStr(key)

	var cpayload C.outbound_redis_payload_t

	defer func() {
		C.outbound_redis_string_free(&caddr)
		C.outbound_redis_string_free(&ckey)
		C.outbound_redis_payload_free(&cpayload)
	}()

	err := C.outbound_redis_get(&caddr, &ckey, &cpayload)
	payload := C.GoBytes(unsafe.Pointer(cpayload.ptr), C.int(cpayload.len))
	return payload, toErr(err)
}

func set(addr, key string, payload []byte) error {
	caddr := redisStr(addr)
	ckey := redisStr(key)
	cpayload := C.outbound_redis_payload_t{ptr: &payload[0], len: C.size_t(len(payload))}

	defer func() {
		C.outbound_redis_string_free(&caddr)
		C.outbound_redis_string_free(&ckey)
		C.outbound_redis_payload_free(&cpayload)
	}()

	err := C.outbound_redis_set(&caddr, &ckey, &cpayload)
	return toErr(err)
}

func redisStr(x string) C.outbound_redis_string_t {
	return C.outbound_redis_string_t{ptr: C.CString(x), len: C.size_t(len(x))}
}

func toErr(code C.uint8_t) error {
	if code == 1 {
		return errors.New("internal server error")
	}
	return nil
}
