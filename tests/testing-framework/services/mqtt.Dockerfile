# Mqtt server with pre-configured user/pass authentication.
FROM eclipse-mosquitto:2
RUN <<EOF
# Create predefined hashed credentials (user, password) for Mosquitto and configure it.
echo -e "user:\$7\$101\$myOLhDLxXIIyi7Sq\$BmKGb8smWSeWrf3Rr5Ee8MefZiPMm1EiKk+RL4BngWFjPn+P0l2t56AJi+NnoGGKyPBDv/lLLUklRwT/GNPnQA==\n" > mosquitto/config/credentials.txt
chmod 0700 mosquitto/config/credentials.txt
# allow_anonymous is enabled with credentials to test both use cases.
echo -e "listener 1883\nallow_anonymous true\npassword_file mosquitto/config/credentials.txt\nlog_type all\n" > mosquitto/config/mosquitto.conf
EOF

EXPOSE 1883