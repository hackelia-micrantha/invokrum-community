_invokrum_complete() {
  local current command
  current="${COMP_WORDS[COMP_CWORD]}"
  command="${COMP_WORDS[1]}"

  if [[ ${COMP_CWORD} -eq 1 ]]; then
    COMPREPLY=( $(compgen -W "validate compose inspect lock verify diff install help version --help --version" -- "$current") )
    return
  fi

  case "$command" in
    validate)
      COMPREPLY=( $(compgen -W "--pack --profile --format --no-color --help" -- "$current") )
      ;;
    compose|lock)
      COMPREPLY=( $(compgen -W "--pack --profile --output --force --no-color --help" -- "$current") )
      ;;
    inspect)
      COMPREPLY=( $(compgen -W "--pack --profile --format --no-color --help" -- "$current") )
      ;;
    verify)
      COMPREPLY=( $(compgen -W "--lock --pack --profile --format --no-color --help" -- "$current") )
      ;;
    diff)
      COMPREPLY=( $(compgen -W "--format --no-color --help" -- "$current") )
      ;;
    install)
      COMPREPLY=( $(compgen -W "--bundle-manifest --subject --candidate-format --store --format --publisher-public-key --publisher-signature --trusted-key-sha256 --no-color --help directory ustar human json" -- "$current") )
      ;;
  esac
}

complete -F _invokrum_complete invokrum
