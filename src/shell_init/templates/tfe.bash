{{SENTINEL}}
tfe() {
    local line
    while IFS= read -r line; do
        case "$line" in
            source:*) source "${line#source:}" ;;
            ?*)       cd "$line" ;;
        esac
    done < <(command tfe "$@")
}
