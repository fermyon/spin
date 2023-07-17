package redis

import (
	"errors"

	reactor "github.com/fermyon/spin/sdk/go/generated"
)

// //export spin_redis_handle_redis_message
// func handleRedisMessage(payload *C.spin_redis_payload_t) C.spin_redis_error_t {
// 	bytes := C.GoBytes(unsafe.Pointer(payload.ptr), C.int(payload.len))
// 	if err := handler(bytes); err != nil {
// 		return C.uint8_t(1)

// 	}
// 	return C.uint8_t(0)
// }

func publish(addr, channel string, payload []byte) error {
	res := reactor.FermyonSpinRedisPublish(addr, channel, payload)
	if res.IsErr() {
		return toErr(res.UnwrapErr())
	}
	return nil
}

func get(addr, key string) ([]byte, error) {
	res := reactor.FermyonSpinRedisGet(addr, key)
	if res.IsOk() {
		return res.Unwrap(), nil
	}
	return nil, toErr(res.UnwrapErr())
}

func set(addr, key string, payload []byte) error {
	res := reactor.FermyonSpinRedisSet(addr, key, payload)
	if res.IsErr() {
		return toErr(res.UnwrapErr())
	}
	return nil
}

func incr(addr, key string) (int64, error) {
	res := reactor.FermyonSpinRedisIncr(addr, key)
	if res.IsOk() {
		return res.Unwrap(), nil
	}
	return 0, toErr(res.UnwrapErr())
}

func del(addr string, keys []string) (int64, error) {
	res := reactor.FermyonSpinRedisDel(addr, keys)
	if res.IsOk() {
		return res.Unwrap(), nil
	}
	return 0, toErr(res.UnwrapErr())
}

func sadd(addr string, key string, values []string) (int64, error) {
	res := reactor.FermyonSpinRedisSadd(addr, key, values)
	if res.IsOk() {
		return res.Unwrap(), nil
	}
	return 0, toErr(res.UnwrapErr())
}

func smembers(addr string, key string) ([]string, error) {
	res := reactor.FermyonSpinRedisSmembers(addr, key)
	if res.IsOk() {
		return res.Unwrap(), nil
	}
	return nil, toErr(res.UnwrapErr())
}

func srem(addr string, key string, values []string) (int64, error) {
	res := reactor.FermyonSpinRedisSrem(addr, key, values)
	if res.IsOk() {
		return res.Unwrap(), nil
	}
	return 0, toErr(res.UnwrapErr())
}

type RedisParameterKind int

const (
	RedisParameterKindInt64 = iota
	RedisParameterKindBinary
)

type RedisParameter struct {
	Kind RedisParameterKind
	Val  interface{}
}

type RedisResultKind int

const (
	RedisResultKindNil = iota
	RedisResultKindStatus
	RedisResultKindInt64
	RedisResultKindBinary
)

type RedisResult struct {
	Kind RedisResultKind
	Val  interface{}
}

func execute(addr string, command string, arguments []RedisParameter) ([]RedisResult, error) {
	res := reactor.FermyonSpinRedisExecute(addr, command, mapParameters(arguments))
	if res.IsOk() {
		return mapResults(res.Unwrap()), nil
	}
	return nil, toErr(res.UnwrapErr())
}

func toErr(err reactor.FermyonSpinRedisError) error {
	switch err.Kind() {
	case reactor.FermyonSpinRedisTypesErrorKindSuccess:
		return nil
	case reactor.FermyonSpinRedisTypesErrorKindError:
		return errors.New("error")
	default:
		return nil
	}
}

func mapParameters(parameters []RedisParameter) []reactor.FermyonSpinRedisTypesRedisParameter {
	r := make([]reactor.FermyonSpinRedisTypesRedisParameter, len(parameters))
	for _, res := range parameters {
		switch res.Kind {
		case RedisParameterKindInt64:
			r = append(r, reactor.FermyonSpinRedisTypesRedisParameterInt64(res.Val.(int64)))
		case RedisParameterKindBinary:
			r = append(r, reactor.FermyonSpinRedisTypesRedisParameterBinary(res.Val.([]uint8)))
		}
	}
	return r

}
func mapResults(results []reactor.FermyonSpinRedisTypesRedisResult) []RedisResult {
	r := make([]RedisResult, len(results))
	for _, res := range results {
		switch res.Kind() {
		case reactor.FermyonSpinRedisTypesRedisResultKindNil:
			r = append(r, RedisResult{Kind: RedisResultKindNil})
		case reactor.FermyonSpinRedisTypesRedisResultKindInt64:
			r = append(r, RedisResult{Kind: RedisResultKindInt64, Val: res.GetInt64()})
		case reactor.FermyonSpinRedisTypesRedisResultKindBinary:
			r = append(r, RedisResult{Kind: RedisResultKindBinary, Val: res.GetBinary()})
		}
	}
	return r
}
