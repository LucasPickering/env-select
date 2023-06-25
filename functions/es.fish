function es --description "Fish wrapper for env-select"
    # Check env-select is installed
    if not type -q env-select
        echo "`env-select` command not available in PATH. See installation instructions:" >&2
        echo "https://github.com/LucasPickering/env-select#installation" >&2
        return 127
    end

    # `set` subcommand is the only one that can generate sourceable output
    if test "$argv[1]" = "set"
        # There are two cases, based on exit code:
        #  - Success: source the output
        #  - Failure: Do nothing, the error should be in stderr already

        # We have to do the source as a separate command so we can access the
        # exit code. "$()" syntax prevents fish from splitting lines into an array
        set output "$(env-select $argv)"
        set return_code $status
        if test $status -eq 0
            echo $output | source
        end
        return $return_code
    else
        env-select $argv
        return $status
    end
end
