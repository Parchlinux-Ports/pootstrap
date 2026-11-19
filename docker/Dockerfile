FROM archlinux:base-devel

LABEL maintainer="Parch Linux Developers <developers@parchlinux.com>"
LABEL description="Parch Linux Mobile bootstrap toolkit container"

RUN pacman -Syu --noconfirm && \
    pacman -S --noconfirm --needed \
        git \
        curl \
        rust \
        cargo \
        arch-install-scripts \
        qemu-user-static \
        qemu-user-static-binfmt \
        qemu-system-aarch64 \
        parted \
        dosfstools \
        e2fsprogs \
        mtools \
        squashfs-tools \
        xorriso \
        sudo \
    && rm -rf /var/cache/pacman/pkg/*

WORKDIR /src
COPY . .

RUN cargo build --release && \
    cp target/release/pootstrap /usr/local/bin/pootstrap && \
    cargo clean && \
    rm -rf /src/*

WORKDIR /work

ENTRYPOINT ["/usr/local/bin/pootstrap"]
CMD ["--help"]
