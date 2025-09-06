#!/usr/bin/env bash
# Usage: bash remote.sh [options]

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
    echo "$user ALL=(ALL) NOPASSWD: ALL" >> /etc/sudoers.d/$user
done

# 各ユーザにRustをインストールする
for user in "${USERS[@]}"; do
    su - $user -c "curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
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
done

# 各ユーザにicfpc2025をcloneする
for user in "${USERS[@]}"; do
    if [ ! -d /home/$user/icfpc2025 ]; then
        su - $user -c "git clone git@github.com:imos/icfpc2025.git ~/icfpc2025"
    fi
done
