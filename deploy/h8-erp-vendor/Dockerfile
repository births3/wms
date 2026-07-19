# 使用本机已有的 debian:bookworm-slim，避免拉取 docker.io/python 受代理阻断。
FROM debian:bookworm-slim

RUN apt-get update \
    && DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends python3 \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY server.py /app/server.py

ENV ERP_VENDOR_HOST=0.0.0.0 \
    ERP_VENDOR_PORT=8080 \
    ERP_VENDOR_DATA=/data \
    ERP_VENDOR_CODE=container-erp-vendor-a \
    ERP_FAIL_COUNT=0 \
    PYTHONUNBUFFERED=1

VOLUME ["/data"]
EXPOSE 8080

HEALTHCHECK --interval=5s --timeout=3s --retries=12 --start-period=8s \
  CMD python3 -c "import urllib.request; urllib.request.urlopen('http://127.0.0.1:8080/healthz', timeout=2)"

CMD ["python3", "/app/server.py"]
