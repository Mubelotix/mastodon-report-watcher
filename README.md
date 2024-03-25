## mastodon-report-watcher

Service providers in the European Union are required to moderate content in under 24 hours. To make sure I don't go in jail in case I'm not able to moderate content in time, I've created this script that checks for reports on my Mastodon instance and shuts down the instance when the deadline is reached before the report is resolved.

### Config

Please add this line to your /etc/sudoers file, after replacing `user` and `mastodon-service` with the appropriate values:

```
user ALL=(ALL) NOPASSWD: /usr/bin/systemctl stop mastodon-service
```
