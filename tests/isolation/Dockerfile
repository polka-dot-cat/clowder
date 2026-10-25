FROM python:3.11-slim-bookworm
RUN apt-get update -qq && apt-get install -y -qq --no-install-recommends \
    git git-lfs build-essential curl ca-certificates iproute2 net-tools util-linux procps \
    && rm -rf /var/lib/apt/lists/*
COPY requirements.txt /tmp/requirements.txt
RUN python -m pip install --no-cache-dir 'pip<25' setuptools==67.8.0 wheel==0.40.0 \
    && python -m pip install --no-cache-dir -r /tmp/requirements.txt
RUN curl --proto '=https' --tlsv1.2 -fsS https://sh.rustup.rs -o /tmp/rustup.sh \
    && sh /tmp/rustup.sh -y --no-modify-path --profile minimal --default-toolchain 1.99.1
ENV PATH="/root/.cargo/bin:${PATH}" GIT_TERMINAL_PROMPT=0 RUSTUP_TOOLCHAIN=1.99.1 CLOWDER_ISOLATED_TESTS=1
COPY tests/isolation/fixtures.json tests/isolation/prepare.py /opt/fixtures/
RUN python /opt/fixtures/prepare.py
WORKDIR /work
COPY . .
RUN python -m pip install --no-deps --no-build-isolation -e . \
    && python -m unittest discover -s tests/isolation -p 'test_*.py' \
    && cargo test --locked && cargo build --release --locked
ENV CLOWDER_IMPLEMENTATION=rust CLOWDER_EXECUTABLE=/work/target/release/clowder PYTHONPATH=/work
ENTRYPOINT ["bash", "/work/tests/isolation/run.sh"]
