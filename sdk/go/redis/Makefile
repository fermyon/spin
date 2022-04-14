.PHONY: generate
generate:
	wit-bindgen c --export ../../../wit/ephemeral/spin-redis.wit
	wit-bindgen c --import ../../../wit/ephemeral/outbound-redis.wit

.PHONY: clean
clean:
	rm spin-redis.h spin-redis.c outbound-redis.h outbound-redis.c
