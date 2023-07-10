es () {
    ENV_SELECT_BINARY test $argv > /dev/null 2>&1
    if [ $? -eq 0 ]; then
        output=$(ENV_SELECT_BINARY $argv)
        return_code=$?
        if [ $return_code -eq 0 ]; then
            source <(echo $output)
        fi
        return $return_code
    else
        ENV_SELECT_BINARY $argv
        return $?
    fi
}
