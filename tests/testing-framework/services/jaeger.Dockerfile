FROM jaegertracing/all-in-one:1.55

ENV COLLECTOR_OTLP_ENABLED=true

HEALTHCHECK --interval=2s CMD wget -qO - http://localhost:14269/health
