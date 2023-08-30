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

	err := C.outbound_redis_publish(&caddr, &cchannel, &cpayload)
	return toErr(err)
}

func get(addr, key string) ([]byte, error) {
	caddr := redisStr(addr)
	ckey := redisStr(key)

	var cpayload C.outbound_redis_payload_t

	err := C.outbound_redis_get(&caddr, &ckey, &cpayload)
	payload := C.GoBytes(unsafe.Pointer(cpayload.ptr), C.int(cpayload.len))
	return payload, toErr(err)
}

func set(addr, key string, payload []byte) error {
	caddr := redisStr(addr)
	ckey := redisStr(key)
	cpayload := C.outbound_redis_payload_t{ptr: &payload[0], len: C.size_t(len(payload))}

	err := C.outbound_redis_set(&caddr, &ckey, &cpayload)
	return toErr(err)
}

func incr(addr, key string) (int64, error) {
	caddr := redisStr(addr)
	ckey := redisStr(key)

	var cpayload C.int64_t

	err := C.outbound_redis_incr(&caddr, &ckey, &cpayload)
	return int64(cpayload), toErr(err)
}

func del(addr string, keys []string) (int64, error) {
	caddr := redisStr(addr)
	ckeys := redisListStr(keys)

	var cpayload C.int64_t

	err := C.outbound_redis_del(&caddr, &ckeys, &cpayload)
	return int64(cpayload), toErr(err)
}

func sadd(addr string, key string, values []string) (int64, error) {
	caddr := redisStr(addr)
	ckey := redisStr(key)
	cvalues := redisListStr(values)

	var cpayload C.int64_t

	err := C.outbound_redis_sadd(&caddr, &ckey, &cvalues, &cpayload)
	return int64(cpayload), toErr(err)
}

func smembers(addr string, key string) ([]string, error) {
	caddr := redisStr(addr)
	ckey := redisStr(key)

	var cpayload C.outbound_redis_list_string_t

	err := C.outbound_redis_smembers(&caddr, &ckey, &cpayload)
	return fromRedisListStr(&cpayload), toErr(err)
}

func srem(addr string, key string, values []string) (int64, error) {
	caddr := redisStr(addr)
	ckey := redisStr(key)
	cvalues := redisListStr(values)

	var cpayload C.int64_t

	err := C.outbound_redis_srem(&caddr, &ckey, &cvalues, &cpayload)
	return int64(cpayload), toErr(err)
}

type RedisParameterKind uint8

const (
	RedisParameterKindInt64 = iota
	RedisParameterKindBinary
)

type RedisParameter struct {
	Kind RedisParameterKind
	Val interface{}
}

type RedisResultKind uint8

const (
	RedisResultKindNil = iota
	RedisResultKindStatus
	RedisResultKindInt64
	RedisResultKindBinary
)

type RedisResult struct {
	Kind RedisResultKind
	Val interface{}
}

func execute(addr string, command string, arguments []RedisParameter) ([]RedisResult, error) {
	caddr := redisStr(addr)
	ccommand := redisStr(command)
	carguments := redisListParameter(arguments)

	var cpayload C.outbound_redis_list_redis_result_t

	err := C.outbound_redis_execute(&caddr, &ccommand, &carguments, &cpayload)
	return fromRedisListResult(&cpayload), toErr(err)
}

func redisStr(x string) C.outbound_redis_string_t {
	return C.outbound_redis_string_t{ptr: C.CString(x), len: C.size_t(len(x))}
}

func redisListStr(xs []string) C.outbound_redis_list_string_t {
	var cxs []C.outbound_redis_string_t

	for i := 0; i < len(xs); i++ {
		cxs = append(cxs, redisStr(xs[i]))
	}
	return C.outbound_redis_list_string_t{ptr: &cxs[0], len: C.size_t(len(cxs))}
}

func fromRedisListStr(list *C.outbound_redis_list_string_t) []string {
	listLen := int(list.len)
	var result []string

	slice := unsafe.Slice(list.ptr, listLen)
	for i := 0; i < listLen; i++ {
		string := slice[i]
		result = append(result, C.GoStringN(string.ptr, C.int(string.len)))
	}

	return result
}

func redisParameter(x RedisParameter) C.outbound_redis_redis_parameter_t {
	var val C._Ctype_union___9
	switch x.Kind {
	case RedisParameterKindInt64: *(*C.int64_t)(unsafe.Pointer(&val)) = x.Val.(int64)
	case RedisParameterKindBinary: {
		value := x.Val.([]byte)
		payload := C.outbound_redis_payload_t{ptr: &value[0], len: C.size_t(len(value))}
		*(*C.outbound_redis_payload_t)(unsafe.Pointer(&val)) = payload
	}
	}
	return C.outbound_redis_redis_parameter_t{tag: C.uint8_t(x.Kind), val: val}
}

func redisListParameter(xs []RedisParameter) C.outbound_redis_list_redis_parameter_t {
	var cxs []C.outbound_redis_redis_parameter_t

	for i := 0; i < len(xs); i++ {
		cxs = append(cxs, redisParameter(xs[i]))
	}
	return C.outbound_redis_list_redis_parameter_t{ptr: &cxs[0], len: C.size_t(len(cxs))}
}

func fromRedisResult(result *C.outbound_redis_redis_result_t) RedisResult {
	var val interface{}
	switch result.tag {
	case 0: val = nil
	case 1: {
		string := (*C.outbound_redis_string_t)(unsafe.Pointer(&result.val))
		val = C.GoStringN(string.ptr, C.int(string.len))
	}
	case 2: val = int64(*(*C.int64_t)(unsafe.Pointer(&result.val)))
	case 3: {
		payload := (*C.outbound_redis_payload_t)(unsafe.Pointer(&result.val))
		val = C.GoBytes(unsafe.Pointer(payload.ptr), C.int(payload.len))
	}
	}

	return RedisResult{Kind: RedisResultKind(result.tag), Val: val}
}

func fromRedisListResult(list *C.outbound_redis_list_redis_result_t) []RedisResult {
	listLen := int(list.len)
	var result []RedisResult

	slice := unsafe.Slice(list.ptr, listLen)
	for i := 0; i < listLen; i++ {
		result = append(result, fromRedisResult(&slice[i]))
	}

	return result
}

func toErr(code C.uint8_t) error {
	if code == 1 {
		return errors.New("internal server error")
	}
	return nil
}
