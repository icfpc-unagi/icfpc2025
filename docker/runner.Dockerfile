FROM ubuntu:24.04
RUN set -eux; \
    MIRROR="http://asia-northeast1.gce.archive.ubuntu.com/ubuntu/"; \
    for f in /etc/apt/sources.list.d/ubuntu.sources /etc/apt/sources.list; do \
      if [ -f "$f" ]; then \
        sed -i.bak -e "s|http://archive.ubuntu.com/ubuntu/|$MIRROR|g" "$f"; \
      fi; \
    done
RUN apt-get update -qy && apt-get install -qy apt-transport-https ca-certificates gnupg curl
RUN echo "deb [signed-by=/usr/share/keyrings/cloud.google.gpg] http://packages.cloud.google.com/apt cloud-sdk main" \
    | tee -a /etc/apt/sources.list.d/google-cloud-sdk.list \
    && curl https://packages.cloud.google.com/apt/doc/apt-key.gpg \
    | apt-key --keyring /usr/share/keyrings/cloud.google.gpg  add - \
    && apt-get update -y && apt-get install google-cloud-sdk -y
COPY ./secrets/service_account.json /service_account.json
RUN gcloud auth activate-service-account icfpc2022@icfpc-primary.iam.gserviceaccount.com \
        --key-file=/service_account.json \
    && gcloud config set project icfpc-primary
COPY ./scripts/exec.sh /usr/local/bin/exec.sh
RUN chmod +x /usr/local/bin/exec.sh

WORKDIR /work
COPY ./problems /work/problems
COPY ./submissions /work/submissions

ENTRYPOINT ["/usr/local/bin/exec.sh"]
