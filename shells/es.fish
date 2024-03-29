function es --description "Fish wrapper for env-select"
    # Make a tmp file for env-select to dump sourceable output to. --source-file
    # is a hidden flag, so consider it safe to pass it ourselves
    set tmp_file (mktemp)
    ENV_SELECT_BINARY --source-file $tmp_file $argv
    # If env-select was successful, source whatever output it *might have* dumped
    set return_code $status
    if test $return_code -eq 0
        source $tmp_file
    end
    rm $tmp_file
    return $return_code
end
