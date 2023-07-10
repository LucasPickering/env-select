function es --description "Fish wrapper for env-select"
    # Test if argv is a `set` command. If so, we'll capture the output. If not,
    # run the command as normal. We silence output here because we're going to
    # execute the command regardless, so if it's an error it'll show up later.
    ENV_SELECT_BINARY test $argv &> /dev/null

    if test $status -eq 0
        set output (ENV_SELECT_BINARY $argv)
        set return_code $status
        if test $status -eq 0
            string join \n $output | source
        end
        return $return_code
    else
        ENV_SELECT_BINARY $argv
        return $status
    end
end
