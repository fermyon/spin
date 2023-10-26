package redis

// #include "outbound-redis.h"
// #include "spin-redis.h"
// #include<stdlib.h>
import "C"
import (
	"errors"
	"fmt"
	"unsafe"
)

// argumentKind represents a type of a argument for executing a Redis command.
type argumentKind uint8

const (
	argumentKindInt argumentKind = iota
	argumentKindBinary
)

// argument represents an argument for a Redis command.
type argument struct {
	kind argumentKind
	val  any
}

func createParameter(x any) (*argument, error) {
	var p argument
	switch v := x.(type) {
	case int:
		p.kind = argumentKindInt
		p.val = int64(v)
	case int32:
		p.kind = argumentKindInt
		p.val = int64(v)
	case int64:
		p.kind = argumentKindInt
		p.val = v
	case string:
		p.kind = argumentKindBinary
		p.val = []byte(v)
	case []byte:
		p.kind = argumentKindBinary
		p.val = v
	default:
		return &p, fmt.Errorf("unsupported parameter type: %T", x)
	}
	return &p, nil
}

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

func sadd(addr, key string, values []string) (int64, error) {
	caddr := redisStr(addr)
	ckey := redisStr(key)
	cvalues := redisListStr(values)

	var cpayload C.int64_t

	err := C.outbound_redis_sadd(&caddr, &ckey, &cvalues, &cpayload)
	return int64(cpayload), toErr(err)
}

func smembers(addr, key string) ([]string, error) {
	caddr := redisStr(addr)
	ckey := redisStr(key)

	var cpayload C.outbound_redis_list_string_t

	err := C.outbound_redis_smembers(&caddr, &ckey, &cpayload)
	return fromRedisListStr(&cpayload), toErr(err)
}

func srem(addr, key string, values []string) (int64, error) {
	caddr := redisStr(addr)
	ckey := redisStr(key)
	cvalues := redisListStr(values)

	var cpayload C.int64_t

	err := C.outbound_redis_srem(&caddr, &ckey, &cvalues, &cpayload)
	return int64(cpayload), toErr(err)
}

func execute(addr, command string, arguments []*argument) ([]*Result, error) {
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
	if len(xs) == 0 {
		return C.outbound_redis_list_string_t{}
	}
	cxs := make([]C.outbound_redis_string_t, 0, len(xs))
	for i := 0; i < len(xs); i++ {
		cxs = append(cxs, redisStr(xs[i]))
	}
	return C.outbound_redis_list_string_t{ptr: &cxs[0], len: C.size_t(len(cxs))}
}

func fromRedisListStr(list *C.outbound_redis_list_string_t) []string {
	listLen := int(list.len)
	result := make([]string, 0, listLen)

	slice := unsafe.Slice(list.ptr, listLen)
	for i := 0; i < listLen; i++ {
		str := slice[i]
		result = append(result, C.GoStringN(str.ptr, C.int(str.len)))
	}
	return result
}

func redisParameter(x *argument) C.outbound_redis_redis_parameter_t {
	var ret C.outbound_redis_redis_parameter_t
	switch x.kind {
	case argumentKindInt:
		*(*C.int64_t)(unsafe.Pointer(&ret.val)) = x.val.(int64)
	case argumentKindBinary:
		value := x.val.([]byte)
		payload := C.outbound_redis_payload_t{ptr: &value[0], len: C.size_t(len(value))}
		*(*C.outbound_redis_payload_t)(unsafe.Pointer(&ret.val)) = payload
	}
	ret.tag = C.uint8_t(x.kind)
	return ret
}

func redisListParameter(xs []*argument) C.outbound_redis_list_redis_parameter_t {
	if len(xs) == 0 {
		return C.outbound_redis_list_redis_parameter_t{}
	}

	cxs := make([]C.outbound_redis_redis_parameter_t, 0, len(xs))
	for i := 0; i < len(xs); i++ {
		cxs = append(cxs, redisParameter(xs[i]))
	}
	return C.outbound_redis_list_redis_parameter_t{ptr: &cxs[0], len: C.size_t(len(cxs))}
}

func fromRedisResult(result *C.outbound_redis_redis_result_t) *Result {
	var val any
	switch ResultKind(result.tag) {
	case ResultKindNil:
		val = nil
	case ResultKindStatus:
		str := (*C.outbound_redis_string_t)(unsafe.Pointer(&result.val))
		val = C.GoStringN(str.ptr, C.int(str.len))
	case ResultKindInt64:
		val = int64(*(*C.int64_t)(unsafe.Pointer(&result.val)))
	case ResultKindBinary:
		payload := (*C.outbound_redis_payload_t)(unsafe.Pointer(&result.val))
		val = C.GoBytes(unsafe.Pointer(payload.ptr), C.int(payload.len))
	}

	return &Result{Kind: ResultKind(result.tag), Val: val}
}

func fromRedisListResult(list *C.outbound_redis_list_redis_result_t) []*Result {
	listLen := int(list.len)
	result := make([]*Result, 0, listLen)

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
