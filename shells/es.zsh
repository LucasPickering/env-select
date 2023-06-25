es () {
    env-select test $argv > /dev/null 2>&1
    if [ $? -eq 0 ]; then
        output=$(env-select $argv)
        return_code=$?
        if [ $return_code -eq 0 ]; then
            source <(echo $output)
        fi
        return $return_code
    else
        env-select $argv
        return $?
    fi
}
