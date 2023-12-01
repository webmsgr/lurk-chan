FROM gitpod/workspace-rust

USER gitpod
RUN bash -lc "rustup default stable"

USER root
RUN apt update && apt install sqlite3