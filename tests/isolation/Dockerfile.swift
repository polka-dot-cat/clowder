FROM swift:6.4-jammy
RUN apt-get update -qq && apt-get install -y -qq --no-install-recommends \
    python3 python3-pip python3-venv git git-lfs ca-certificates iproute2 net-tools util-linux procps \
    && rm -rf /var/lib/apt/lists/* \
    && python3 -m venv /opt/test-python
ENV PATH="/opt/test-python/bin:${PATH}" GIT_TERMINAL_PROMPT=0 CLOWDER_ISOLATED_TESTS=1
COPY requirements.txt /tmp/requirements.txt
RUN python -m pip install --no-cache-dir 'pip<25' setuptools==67.8.0 wheel==0.40.0 \
    && python -m pip install --no-cache-dir -r /tmp/requirements.txt
COPY tests/isolation/fixtures.json tests/isolation/prepare.py /opt/fixtures/
RUN python /opt/fixtures/prepare.py
WORKDIR /work
COPY . .
RUN python -m pip install --no-deps --no-build-isolation -e . \
    && python -m unittest discover -s tests/isolation -p 'test_*.py' \
    && swift test --package-path swift \
    && swift build --package-path swift -c release --static-swift-stdlib
ENV CLOWDER_IMPLEMENTATION=swift CLOWDER_EXECUTABLE=/work/swift/.build/release/clowder PYTHONPATH=/work
ENTRYPOINT ["bash", "/work/tests/isolation/run.sh"]
