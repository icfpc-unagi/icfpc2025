#!/usr/bin/env bash

set -eux

apt-get update -qq
apt-get install -y make clang cmake docker.io

cd "$(dirname "${BASH_SOURCE}")/.."

USERS=(
    imos iwiwi chokudai toslunar wata-orz sulume ninetan
)

# ユーザを追加する
for user in "${USERS[@]}"; do
    # もしユーザが存在しなければ
    if ! id "$user" &>/dev/null; then
        adduser --disabled-password --gecos '' "$user"
    fi
done

# SSHキーを追加する
for user in "${USERS[@]}"; do
    mkdir -p /home/$user/.ssh
    curl "https://github.com/${user}.keys" >> /home/$user/.ssh/authorized_keys
done

# sudoersに追加する
for user in "${USERS[@]}"; do
    echo "$user ALL=(ALL) NOPASSWD: ALL" > /etc/sudoers.d/icfpc2025-$user
done

# 各ユーザにRustをインストールする
for user in "${USERS[@]}"; do
    sudo -u $user bash -c "curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y"
done

# 各ユーザにid_ed25519を配置する
for user in "${USERS[@]}"; do
    mkdir -p /home/$user/.ssh
    cp secrets/id_ed25519 /home/$user/.ssh/
    chown $user:$user /home/$user/.ssh/id_ed25519
    chmod 600 /home/$user/.ssh/id_ed25519
    cp configs/id_ed25519.pub /home/$user/.ssh/
    chown $user:$user /home/$user/.ssh/id_ed25519.pub
    chmod 644 /home/$user/.ssh/id_ed25519.pub
    touch /home/$user/.ssh/known_hosts
    chown $user:$user /home/$user/.ssh/*
    chmod 644 /home/$user/.ssh/known_hosts
done

# 各ユーザにicfpc2025をcloneする
for user in "${USERS[@]}"; do
    if [ ! -d /home/$user/icfpc2025 ]; then
        sudo -u $user bash -c "GIT_SSH_COMMAND='ssh -o StrictHostKeyChecking=accept-new' git clone git@github.com:icfpc-unagi/icfpc2025.git ~/icfpc2025"
    fi
done
