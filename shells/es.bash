es () {
    ENV_SELECT_BINARY test $@ > /dev/null 2>&1
    if [ $? -eq 0 ]; then
        tmp_file=$(mktemp)
        ENV_SELECT_BINARY $@ > $tmp_file
        return_code=$?
        if [ $return_code -eq 0 ]; then
            source $tmp_file
        fi
        rm $tmp_file
        return $return_code
    else
        ENV_SELECT_BINARY $@
        return $?
    fi
}
