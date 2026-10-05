{{SENTINEL}}
# tfe wrapper: cd to the directory printed by tfe on exit.
# def --env is required so the cd takes effect in the caller's shell.
def --env --wrapped tfe [...rest] {
    let dir = (^tfe ...$rest | str trim)
    if ($dir | is-not-empty) { cd $dir }
}
