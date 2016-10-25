function __fish_using_command
    set cmd (commandline -opc)
    if [ (count $cmd) -eq (count $argv) ]
        for i in (seq (count $argv))
            if [ $cmd[$i] != $argv[$i] ]
                return 1
            end
        end
        return 0
    end
    return 1
end

complete -c git-journal -n '__fish_using_command git-journal' -s p -l path -d 'Sets a custom working path.'
complete -c git-journal -n '__fish_using_command git-journal' -s n -l tags-count -d 'The number of tags until the parser stops when a single revision is given.'
complete -c git-journal -n '__fish_using_command git-journal' -s e -d 'A pattern to exclude git tags from the processing.'
complete -c git-journal -n '__fish_using_command git-journal' -s t -l template -d 'Use a custom output template.'
complete -c git-journal -n '__fish_using_command git-journal' -s o -l output -d 'The output file for the changelog.'
complete -c git-journal -n '__fish_using_command git-journal' -s a -l all -d 'Do not stop parsing at the first tag when a single revision is given. Overwrites '-n/--tags-count'.'
complete -c git-journal -n '__fish_using_command git-journal' -s g -l generate -d 'Generate a fresh output template from a commit range.'
complete -c git-journal -n '__fish_using_command git-journal' -s s -l short -d 'Print only the shortlog (summary) form.'
complete -c git-journal -n '__fish_using_command git-journal' -s u -l skip-unreleased -d 'Skip entries without any relation to a git TAG.'
complete -c git-journal -n '__fish_using_command git-journal' -s h -l help -d 'Prints help information'
complete -c git-journal -n '__fish_using_command git-journal' -s V -l version -d 'Prints version information'
complete -c git-journal -n '__fish_using_command git-journal' -s h -l help -d 'Prints help information'
complete -c git-journal -n '__fish_using_command git-journal' -s V -l version -d 'Prints version information'
complete -c git-journal -n '__fish_using_command git-journal' -f -a 'prepare'
complete -c git-journal -n '__fish_using_command git-journal' -f -a 'setup'
complete -c git-journal -n '__fish_using_command git-journal' -f -a 'verify'
complete -c git-journal -n '__fish_using_command git-journal' -f -a 'help'
complete -c git-journal -n '__fish_using_command git-journal' -f -a 'help'
complete -c git-journal -n '__fish_using_command git-journal prepare' -s h -l help -d 'Prints help information'
complete -c git-journal -n '__fish_using_command git-journal prepare' -s V -l version -d 'Prints version information'
complete -c git-journal -n '__fish_using_command git-journal prepare' -s h -l help -d 'Prints help information'
complete -c git-journal -n '__fish_using_command git-journal prepare' -s V -l version -d 'Prints version information'
complete -c git-journal -n '__fish_using_command git-journal setup' -s h -l help -d 'Prints help information'
complete -c git-journal -n '__fish_using_command git-journal setup' -s V -l version -d 'Prints version information'
complete -c git-journal -n '__fish_using_command git-journal setup' -s h -l help -d 'Prints help information'
complete -c git-journal -n '__fish_using_command git-journal setup' -s V -l version -d 'Prints version information'
complete -c git-journal -n '__fish_using_command git-journal verify' -s h -l help -d 'Prints help information'
complete -c git-journal -n '__fish_using_command git-journal verify' -s V -l version -d 'Prints version information'
complete -c git-journal -n '__fish_using_command git-journal verify' -s h -l help -d 'Prints help information'
complete -c git-journal -n '__fish_using_command git-journal verify' -s V -l version -d 'Prints version information'
complete -c git-journal -n '__fish_using_command git-journal help' -s h -l help -d 'Prints help information'
complete -c git-journal -n '__fish_using_command git-journal help' -s V -l version -d 'Prints version information'
complete -c git-journal -n '__fish_using_command git-journal help' -s h -l help -d 'Prints help information'
complete -c git-journal -n '__fish_using_command git-journal help' -s V -l version -d 'Prints version information'
complete -c git-journal -n '__fish_using_command git-journal help' -s h -l help -d 'Prints help information'
complete -c git-journal -n '__fish_using_command git-journal help' -s V -l version -d 'Prints version information'
