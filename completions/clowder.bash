# Native Clowder completion for Bash 3.2 and later.
_clowder_native_complete() {
    local candidate
    COMPREPLY=()
    # Never pass partial commands to an older Python executable on PATH.
    if [[ $(CLOWDER_COMPLETE=probe "${COMP_WORDS[0]}" --_clowder-completion-probe 2>/dev/null) != clowder-native-completion-v1 ]]; then
        return
    fi
    while IFS= read -r candidate; do
        COMPREPLY+=("$candidate")
    done < <(CLOWDER_COMPLETE=1 "${COMP_WORDS[0]}" "${COMP_WORDS[@]:1:COMP_CWORD}")
}
complete -o default -F _clowder_native_complete clowder
