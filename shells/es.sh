es () {
    # Make a tmp file for env-select to dump sourceable output to. --source-file
    # is a hidden flag, so consider it safe to pass it ourselves
    tmp_file=$(mktemp)
    ENV_SELECT_BINARY --source-file $tmp_file $@
    # If env-select was successful, source whatever output it *might have* dumped
    return_code=$?
    if [ $return_code -eq 0 ]; then
        source $tmp_file
    fi
    rm $tmp_file
    return $return_code
}
