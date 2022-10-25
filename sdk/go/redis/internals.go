package redis

// #include "outbound-redis.h"
// #include "spin-redis.h"
// #include<stdlib.h>
import "C"
import (
	"errors"
	"unsafe"
)

//export spin_redis_handle_redis_message
func handleRedisMessage(payload *C.spin_redis_payload_t) C.spin_redis_error_t {
	bytes := C.GoBytes(unsafe.Pointer(payload.ptr), C.int(payload.len))
	if err := handler(bytes); err != nil {
		return C.uint8_t(1)

	}
	return C.uint8_t(0)
}

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

func incr(addr, key string) (int64, error) {
	caddr := redisStr(addr)
	ckey := redisStr(key)

	var cpayload C.int64_t

	defer func() {
		C.outbound_redis_string_free(&caddr)
		C.outbound_redis_string_free(&ckey)
	}()

	err := C.outbound_redis_incr(&caddr, &ckey, &cpayload)
	return int64(cpayload), toErr(err)
}

func del(addr string, keys []string) (int64, error) {
	caddr := redisStr(addr)
	ckeys := redisListStr(keys)

	var cpayload C.int64_t

	defer func() {
		C.outbound_redis_string_free(&caddr)
		C.outbound_redis_list_string_free(&ckeys)
	}()

	err := C.outbound_redis_del(&caddr, &ckeys, &cpayload)
	return int64(cpayload), toErr(err)
}

func redisListStr(xs []string) C.outbound_redis_list_string_t {
	var cxs []C.outbound_redis_string_t

	for i := 0; i < len(xs); i++ {
		cxs = append(cxs, redisStr(xs[i]))
	}
	return C.outbound_redis_list_string_t{ptr: &cxs[0], len: C.size_t(len(cxs))}
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
