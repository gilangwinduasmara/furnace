[global]
include=NONE
pid = {php_dir}/php-fpm.pid
error_log = {php_dir}/php-fpm.log

[www]
user = {user}
group = {group}
listen = {sock_path}
listen.owner = {user}
listen.group = {group}
listen.mode = 0660
pm = dynamic
pm.max_children = 5
pm.start_servers = 2
pm.min_spare_servers = 1
pm.max_spare_servers = 3
chdir = /
