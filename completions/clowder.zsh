# Native Clowder completion. Source after running compinit.
_clowder_native_complete() {
    local -a candidates
    if [[ $(CLOWDER_COMPLETE=probe "${words[1]}" --_clowder-completion-probe 2>/dev/null) != clowder-native-completion-v1 ]]; then
        _files
        return
    fi
    candidates=("${(@f)$(CLOWDER_COMPLETE=1 "${words[1]}" "${words[@]:1:$((CURRENT - 1))}")}")
    if (( ${#candidates} )) && [[ -n ${candidates[1]} ]]; then
        compadd -- "${candidates[@]}"
    else
        _files
    fi
}
compdef _clowder_native_complete clowder
