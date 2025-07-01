#!/bin/bash

echo "=== 1. Check PHP-FPM socket ==="
ls -l ~/.furnace/php/8.2/php-fpm.sock

echo
echo "=== 2. Check PHP-FPM process ==="
pgrep -fl php-fpm

echo
echo "=== 3. Check Nginx config for project ==="
cat ~/.furnace/nginx/servers/reservation-online-sales-agent.conf

echo
echo "=== 4. Check Nginx process ==="
ps aux | grep '[n]ginx'

echo
echo "=== 5. Check DNS resolution ==="
dig +short reservation-online-sales-agent.test

echo
echo "=== 6. Curl the site (force 127.0.0.1) ==="
curl -v --resolve reservation-online-sales-agent.test:80:127.0.0.1 http://reservation-online-sales-agent.test/

echo
echo "=== 7. Check Nginx error log ==="
tail -n 20 ~/.furnace/nginx/logs/reservation-online-sales-agent.error.log

echo
echo "=== 8. Check PHP-FPM log ==="
tail -n 20 ~/.furnace/php/8.2/php-fpm.log

echo
echo "=== 9. Check Laravel public/index.php exists ==="
ls -l /Users/gilangwinduasmara/Projects/Bambulogy/reservation-online-sales-agent/public/index.php