{{SENTINEL}}
function tfe {
    & (Get-Command tfe -CommandType Application).Source @args | ForEach-Object {
        if ($_ -like 'source:*') { . ($_ -replace '^source:', '') }
        elseif ($_)              { Set-Location $_ }
    }
}
