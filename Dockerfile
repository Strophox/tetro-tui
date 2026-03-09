FROM rust:slim
ENV TERM=xterm-color256
WORKDIR /usr/games/tetrs
COPY . .
RUN cargo build
LABEL org.opencontainers.image.source=https://github.com/Strophox/tetrs
ENTRYPOINT ["/usr/games/tetrs/target/debug/tetrs_terminal"]
