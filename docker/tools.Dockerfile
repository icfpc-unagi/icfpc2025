FROM ubuntu:24.04
RUN sed -i.bak -e "s%http://archive.ubuntu.com/ubuntu/%http://asia-northeast1.gce.archive.ubuntu.com/ubuntu/%g" \
    /etc/apt/sources.list
ADD bin/apt-install /usr/local/bin/apt-install
RUN apt-install openssl make jq curl ca-certificates
