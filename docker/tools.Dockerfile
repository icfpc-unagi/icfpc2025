FROM ubuntu:24.04
RUN set -eux; \
    MIRROR="http://asia-northeast1.gce.archive.ubuntu.com/ubuntu/"; \
    for f in /etc/apt/sources.list.d/ubuntu.sources /etc/apt/sources.list; do \
      if [ -f "$f" ]; then \
        sed -i.bak -e "s|http://archive.ubuntu.com/ubuntu/|$MIRROR|g" "$f"; \
      fi; \
    done
ADD bin/apt-install /usr/local/bin/apt-install
RUN apt-install openssl make jq curl ca-certificates
