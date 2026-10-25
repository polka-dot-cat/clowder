#!/usr/bin/env bash
# Run only inside the image; networking and Git configuration changes are local.
set -euo pipefail
[[ -f /.dockerenv && -d /fixtures ]] || { echo 'Use script/test-isolated on the host' >&2; exit 2; }
cd /work
git config --global user.name 'Clowder isolated verification'
git config --global user.email 'joe@polka.cat'
git config --global protocol.file.allow always
export GIT_SSH_COMMAND='python /work/tests/isolation/ssh.py'
export GIT_TERMINAL_PROMPT=0
suite="${1:?Missing suite}"
selection=()
tests=tests/functional
case "$suite" in
    read) markers='not ssh and not offline and not write and not lfs' ;;
    write)
        markers='write and not offline'
        # This suite runs with --network none. All remote mutations are local.
        for name in cats mu duke kishka kit sasha june; do
            for url in "git@github.com:polka-dot-cat/$name.git" "https://github.com/polka-dot-cat/$name.git"; do
                git config --global --add "url.file:///fixtures/polka-dot-cat/$name.git.insteadOf" "$url"
            done
        done
        ip address add 8.8.8.8/32 dev lo
        python tests/isolation/connectivity.py &
        connectivity_pid=$!
        trap 'kill "$connectivity_pid" 2>/dev/null || true' EXIT
        for attempt in {1..20}; do
            if (echo > /dev/tcp/8.8.8.8/53) 2>/dev/null; then break; fi
            sleep 0.1
        done
        ;;
    schema|schema-lfs|source-urls)
        tests=tests/functional/test_schema_validation.py
        selection=(-k 'not source_url or implicit')
        markers='ssh and not lfs'
        if [[ "$suite" == schema-lfs ]]; then markers='ssh and lfs'; fi
        if [[ "$suite" == source-urls ]]; then
            selection=(-k 'source_url and not implicit')
            export FIXTURE_CUSTOM_URLS=1
        fi
        ;;
    ssh-features)
        tests=tests/functional/test_features.py
        markers='ssh and not offline and not write and not lfs'
        export FIXTURE_WRITES=1
        ;;
    offline|offline-write)
        tests=tests/functional/test_features.py
        markers='offline and not write'
        export FIXTURE_OFFLINE=1 FIXTURE_WRITES=1
        if [[ "$suite" == offline-write ]]; then markers='offline and write'; fi
        ;;
    lfs)
        markers='lfs and not ssh and not offline and not write'
        for filter in clean smudge process required; do
            git config --system --unset-all "filter.lfs.$filter" || true
        done
        ;;
    *) echo "Unknown suite: $suite" >&2; exit 2 ;;
esac
if [[ "$suite" != lfs && "$suite" != schema-lfs ]]; then export GIT_LFS_SKIP_SMUDGE=1; fi
mkdir -p "/reports/$suite"
python -m pytest "$tests" -q --tb=short --show-capture=no --maxfail=5 \
    --basetemp="/test-$suite" -m "$markers" "${selection[@]}" \
    --junitxml="/reports/$suite/results.xml" --cucumberjson="/reports/$suite/results.json"
