#!/bin/bash

# Wait for Kafka to be ready
echo "Waiting for Kafka to be ready..."
while ! /opt/kafka/bin/kafka-broker-api-versions.sh --bootstrap-server kafka:9092 >/dev/null 2>&1; do
    echo "Kafka not ready, sleeping for 2 seconds..."
    sleep 2
done

echo "Kafka is ready! Creating topics..."

# Create order topics
/opt/kafka/bin/kafka-topics.sh --create --topic order.created --bootstrap-server kafka:9092 --partitions 3 --replication-factor 1 --if-not-exists
/opt/kafka/bin/kafka-topics.sh --create --topic order.updated --bootstrap-server kafka:9092 --partitions 3 --replication-factor 1 --if-not-exists
/opt/kafka/bin/kafka-topics.sh --create --topic order.deleted --bootstrap-server kafka:9092 --partitions 3 --replication-factor 1 --if-not-exists

# Create email service topics (existing ones)
/opt/kafka/bin/kafka-topics.sh --create --topic email-service-topic-auth-forgot-password --bootstrap-server kafka:9092 --partitions 3 --replication-factor 1 --if-not-exists
/opt/kafka/bin/kafka-topics.sh --create --topic email-service-topic-auth-register --bootstrap-server kafka:9092 --partitions 3 --replication-factor 1 --if-not-exists
/opt/kafka/bin/kafka-topics.sh --create --topic email-service-topic-auth-verify-code-success --bootstrap-server kafka:9092 --partitions 3 --replication-factor 1 --if-not-exists

echo "All topics created successfully!"

# List all topics to verify
echo "Current topics:"
/opt/kafka/bin/kafka-topics.sh --list --bootstrap-server kafka:9092