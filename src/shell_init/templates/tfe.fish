{{SENTINEL}}
function tfe
    for line in (command tfe $argv | string split0 | string split "\n")
        if string match -q 'source:*' -- $line
            source (string replace 'source:' '' -- $line)
        else if test -n "$line"
            cd $line
        end
    end
end
