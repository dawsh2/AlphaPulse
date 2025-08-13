# AlphaPulse Production Deployment Pipeline

## 🎯 Overview

This document defines the comprehensive production deployment strategy for AlphaPulse's ultra-low latency trading infrastructure, ensuring zero-downtime deployments while maintaining sub-10μs performance targets.

## 🏗️ Deployment Architecture

### **Multi-Environment Strategy**

```
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│   Development   │───▶│     Staging     │───▶│   Production    │
│                 │    │                 │    │                 │
│ • Local testing │    │ • Integration   │    │ • Live trading  │
│ • Unit tests    │    │ • Performance   │    │ • High SLA      │
│ • Hot reload    │    │ • Load testing  │    │ • Monitoring    │
└─────────────────┘    └─────────────────┘    └─────────────────┘
```

### **Infrastructure Components**

```
┌───────────────────────────────────────────────────────────────┐
│                     Production Infrastructure                 │
├───────────────────────────────────────────────────────────────┤
│                                                               │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐           │
│  │   Load      │  │   Load      │  │   Load      │           │
│  │ Balancer    │  │ Balancer    │  │ Balancer    │           │
│  │ (Coinbase)  │  │ (Kraken)    │  │ (Binance)   │           │
│  └──────┬──────┘  └──────┬──────┘  └──────┬──────┘           │
│         │                │                │                  │
│  ┌──────▼──────┐  ┌──────▼──────┐  ┌──────▼──────┐           │
│  │ Collector   │  │ Collector   │  │ Collector   │           │
│  │ Pod 1       │  │ Pod 1       │  │ Pod 1       │           │
│  │ Pod 2       │  │ Pod 2       │  │ Pod 2       │           │
│  └──────┬──────┘  └──────┬──────┘  └──────┬──────┘           │
│         │                │                │                  │
│         └────────────────┼────────────────┘                  │
│                          │                                   │
│                   ┌──────▼──────┐                            │
│                   │ Shared      │                            │
│                   │ Memory      │                            │
│                   │ Volume      │                            │
│                   └──────┬──────┘                            │
│                          │                                   │
│    ┌─────────────────────┼─────────────────────┐             │
│    │                     │                     │             │
│ ┌──▼──────┐        ┌─────▼─────┐        ┌──────▼──────┐     │
│ │WebSocket│        │ Python    │        │ API Server  │     │
│ │Server   │        │ Bindings  │        │ Cluster     │     │
│ │Cluster  │        │ Service   │        │             │     │
│ └─────────┘        └───────────┘        └─────────────┘     │
└───────────────────────────────────────────────────────────────┘
```

## 🐳 Containerization Strategy

### **Multi-Stage Docker Builds**

```dockerfile
# Dockerfile.collector
FROM rust:1.75-alpine AS builder

# Install dependencies
RUN apk add --no-cache musl-dev pkgconfig openssl-dev

# Set working directory
WORKDIR /usr/src/app

# Copy and cache dependencies
COPY Cargo.toml Cargo.lock ./
COPY common ./common
COPY collectors ./collectors

# Build optimized release
RUN cargo build --release --bin alphapulse-collectors

# Runtime stage
FROM alpine:latest

# Install runtime dependencies
RUN apk add --no-cache ca-certificates libgcc

# Create non-root user
RUN addgroup -g 1000 alphapulse && \
    adduser -D -s /bin/sh -u 1000 -G alphapulse alphapulse

# Create shared memory directory
RUN mkdir -p /tmp/alphapulse_shm && \
    chown alphapulse:alphapulse /tmp/alphapulse_shm

# Copy binary
COPY --from=builder /usr/src/app/target/release/alphapulse-collectors /usr/local/bin/

# Switch to non-root user
USER alphapulse

# Health check
HEALTHCHECK --interval=10s --timeout=5s --retries=3 \
  CMD curl -f http://localhost:${METRICS_PORT:-8080}/health || exit 1

# Default command
CMD ["alphapulse-collectors"]
```

### **WebSocket Server Container**

```dockerfile
# Dockerfile.websocket
FROM rust:1.75-alpine AS builder

WORKDIR /usr/src/app
COPY Cargo.toml Cargo.lock ./
COPY common ./common
COPY websocket-server ./websocket-server

RUN cargo build --release --bin alphapulse-websocket-server

FROM alpine:latest
RUN apk add --no-cache ca-certificates libgcc
RUN addgroup -g 1000 alphapulse && \
    adduser -D -s /bin/sh -u 1000 -G alphapulse alphapulse

COPY --from=builder /usr/src/app/target/release/alphapulse-websocket-server /usr/local/bin/

USER alphapulse
EXPOSE 8765

HEALTHCHECK --interval=10s --timeout=5s --retries=3 \
  CMD curl -f http://localhost:8766/health || exit 1

CMD ["alphapulse-websocket-server"]
```

### **Python Bindings Service**

```dockerfile
# Dockerfile.python-bindings
FROM python:3.11-slim AS builder

# Install Rust for PyO3 compilation
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
ENV PATH="/root/.cargo/bin:${PATH}"

# Install Python build dependencies
RUN pip install setuptools-rust wheel maturin

WORKDIR /usr/src/app
COPY python-bindings ./

# Build Python wheel
RUN maturin build --release --out dist

# Runtime stage
FROM python:3.11-slim

# Install runtime dependencies
RUN pip install numpy pandas asyncio plotly ipywidgets jupyter

# Copy and install wheel
COPY --from=builder /usr/src/app/dist/*.whl /tmp/
RUN pip install /tmp/*.whl

# Create service user
RUN useradd -m -u 1000 alphapulse

# Create shared memory mount point
RUN mkdir -p /tmp/alphapulse_shm && \
    chown alphapulse:alphapulse /tmp/alphapulse_shm

USER alphapulse
WORKDIR /home/alphapulse

# Default command for service mode
CMD ["python", "-c", "import alphapulse_rust; print('Python bindings service ready')"]
```

## ☸️ Kubernetes Deployment

### **Namespace and RBAC**

```yaml
# k8s/namespace.yaml
apiVersion: v1
kind: Namespace
metadata:
  name: alphapulse
  labels:
    name: alphapulse
    environment: production

---
apiVersion: v1
kind: ServiceAccount
metadata:
  name: alphapulse-service-account
  namespace: alphapulse

---
apiVersion: rbac.authorization.k8s.io/v1
kind: ClusterRole
metadata:
  name: alphapulse-cluster-role
rules:
- apiGroups: [""]
  resources: ["pods", "services", "endpoints"]
  verbs: ["get", "list", "watch"]
- apiGroups: ["apps"]
  resources: ["deployments", "replicasets"]
  verbs: ["get", "list", "watch"]

---
apiVersion: rbac.authorization.k8s.io/v1
kind: ClusterRoleBinding
metadata:
  name: alphapulse-cluster-role-binding
roleRef:
  apiGroup: rbac.authorization.k8s.io
  kind: ClusterRole
  name: alphapulse-cluster-role
subjects:
- kind: ServiceAccount
  name: alphapulse-service-account
  namespace: alphapulse
```

### **Shared Memory Volume**

```yaml
# k8s/shared-memory-volume.yaml
apiVersion: v1
kind: PersistentVolume
metadata:
  name: alphapulse-shared-memory
spec:
  capacity:
    storage: 2Gi
  volumeMode: Filesystem
  accessModes:
    - ReadWriteMany
  persistentVolumeReclaimPolicy: Retain
  storageClassName: high-performance
  hostPath:
    path: /dev/shm/alphapulse
    type: DirectoryOrCreate

---
apiVersion: v1
kind: PersistentVolumeClaim
metadata:
  name: alphapulse-shared-memory-claim
  namespace: alphapulse
spec:
  accessModes:
    - ReadWriteMany
  resources:
    requests:
      storage: 2Gi
  storageClassName: high-performance
```

### **Collector Deployment**

```yaml
# k8s/coinbase-collector.yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: coinbase-collector
  namespace: alphapulse
  labels:
    app: alphapulse-collector
    exchange: coinbase
spec:
  replicas: 2
  strategy:
    type: RollingUpdate
    rollingUpdate:
      maxUnavailable: 0
      maxSurge: 1
  selector:
    matchLabels:
      app: alphapulse-collector
      exchange: coinbase
  template:
    metadata:
      labels:
        app: alphapulse-collector
        exchange: coinbase
      annotations:
        prometheus.io/scrape: "true"
        prometheus.io/port: "8080"
        prometheus.io/path: "/metrics"
    spec:
      serviceAccountName: alphapulse-service-account
      securityContext:
        runAsNonRoot: true
        runAsUser: 1000
        runAsGroup: 1000
        fsGroup: 1000
      affinity:
        podAntiAffinity:
          preferredDuringSchedulingIgnoredDuringExecution:
          - weight: 100
            podAffinityTerm:
              labelSelector:
                matchExpressions:
                - key: app
                  operator: In
                  values: ["alphapulse-collector"]
                - key: exchange
                  operator: In
                  values: ["coinbase"]
              topologyKey: kubernetes.io/hostname
      containers:
      - name: collector
        image: alphapulse/collector:v1.0.0
        imagePullPolicy: IfNotPresent
        env:
        - name: EXCHANGE
          value: "coinbase"
        - name: SYMBOLS
          value: "BTC-USD,ETH-USD,BTC-USDT,ETH-USDT"
        - name: RUST_LOG
          value: "alphapulse_collectors=info,alphapulse_common=info"
        - name: METRICS_PORT
          value: "8080"
        - name: SHARED_MEMORY_PATH
          value: "/tmp/alphapulse_shm"
        ports:
        - containerPort: 8080
          name: metrics
          protocol: TCP
        resources:
          requests:
            memory: "128Mi"
            cpu: "250m"
          limits:
            memory: "256Mi"
            cpu: "500m"
        livenessProbe:
          httpGet:
            path: /health
            port: 8080
          initialDelaySeconds: 30
          periodSeconds: 10
          timeoutSeconds: 5
          failureThreshold: 3
        readinessProbe:
          httpGet:
            path: /ready
            port: 8080
          initialDelaySeconds: 5
          periodSeconds: 5
          timeoutSeconds: 3
          failureThreshold: 3
        volumeMounts:
        - name: shared-memory
          mountPath: /tmp/alphapulse_shm
        securityContext:
          allowPrivilegeEscalation: false
          readOnlyRootFilesystem: true
          capabilities:
            drop:
            - ALL
      volumes:
      - name: shared-memory
        persistentVolumeClaim:
          claimName: alphapulse-shared-memory-claim
      restartPolicy: Always
      terminationGracePeriodSeconds: 30

---
apiVersion: v1
kind: Service
metadata:
  name: coinbase-collector-service
  namespace: alphapulse
  labels:
    app: alphapulse-collector
    exchange: coinbase
spec:
  selector:
    app: alphapulse-collector
    exchange: coinbase
  ports:
  - port: 8080
    targetPort: 8080
    name: metrics
  type: ClusterIP

---
apiVersion: autoscaling/v2
kind: HorizontalPodAutoscaler
metadata:
  name: coinbase-collector-hpa
  namespace: alphapulse
spec:
  scaleTargetRef:
    apiVersion: apps/v1
    kind: Deployment
    name: coinbase-collector
  minReplicas: 2
  maxReplicas: 5
  metrics:
  - type: Resource
    resource:
      name: cpu
      target:
        type: Utilization
        averageUtilization: 70
  - type: Resource
    resource:
      name: memory
      target:
        type: Utilization
        averageUtilization: 80
  behavior:
    scaleUp:
      stabilizationWindowSeconds: 60
      policies:
      - type: Percent
        value: 50
        periodSeconds: 60
    scaleDown:
      stabilizationWindowSeconds: 300
      policies:
      - type: Percent
        value: 10
        periodSeconds: 60
```

### **WebSocket Server Deployment**

```yaml
# k8s/websocket-server.yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: websocket-server
  namespace: alphapulse
  labels:
    app: alphapulse-websocket-server
spec:
  replicas: 3
  strategy:
    type: RollingUpdate
    rollingUpdate:
      maxUnavailable: 1
      maxSurge: 1
  selector:
    matchLabels:
      app: alphapulse-websocket-server
  template:
    metadata:
      labels:
        app: alphapulse-websocket-server
    spec:
      serviceAccountName: alphapulse-service-account
      containers:
      - name: websocket-server
        image: alphapulse/websocket-server:v1.0.0
        env:
        - name: WEBSOCKET_PORT
          value: "8765"
        - name: METRICS_PORT
          value: "8766"
        - name: SHARED_MEMORY_PATH
          value: "/tmp/alphapulse_shm"
        ports:
        - containerPort: 8765
          name: websocket
        - containerPort: 8766
          name: metrics
        resources:
          requests:
            memory: "64Mi"
            cpu: "100m"
          limits:
            memory: "128Mi"
            cpu: "250m"
        livenessProbe:
          httpGet:
            path: /health
            port: 8766
          initialDelaySeconds: 30
          periodSeconds: 10
        readinessProbe:
          httpGet:
            path: /ready
            port: 8766
          initialDelaySeconds: 5
          periodSeconds: 5
        volumeMounts:
        - name: shared-memory
          mountPath: /tmp/alphapulse_shm
          readOnly: true
      volumes:
      - name: shared-memory
        persistentVolumeClaim:
          claimName: alphapulse-shared-memory-claim

---
apiVersion: v1
kind: Service
metadata:
  name: websocket-server-service
  namespace: alphapulse
spec:
  selector:
    app: alphapulse-websocket-server
  ports:
  - port: 8765
    targetPort: 8765
    name: websocket
  - port: 8766
    targetPort: 8766
    name: metrics
  type: LoadBalancer
```

## 🚀 CI/CD Pipeline

### **GitHub Actions Workflow**

```yaml
# .github/workflows/deploy.yml
name: AlphaPulse Deployment Pipeline

on:
  push:
    branches: [main, develop]
  pull_request:
    branches: [main]

env:
  REGISTRY: ghcr.io
  IMAGE_NAME: alphapulse

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
    - uses: actions/checkout@v4
    
    - name: Setup Rust
      uses: actions-rs/toolchain@v1
      with:
        toolchain: stable
        components: rustfmt, clippy
    
    - name: Cache Rust dependencies
      uses: actions/cache@v3
      with:
        path: |
          ~/.cargo/registry
          ~/.cargo/git
          target
        key: ${{ runner.os }}-cargo-${{ hashFiles('**/Cargo.lock') }}
    
    - name: Run tests
      run: |
        cargo test --workspace
        cargo clippy -- -D warnings
        cargo fmt -- --check
    
    - name: Run security audit
      run: |
        cargo install cargo-audit
        cargo audit
    
    - name: Run performance benchmarks
      run: |
        cargo bench --workspace

  build:
    needs: test
    runs-on: ubuntu-latest
    strategy:
      matrix:
        component: [collector, websocket-server, python-bindings]
    steps:
    - uses: actions/checkout@v4
    
    - name: Set up Docker Buildx
      uses: docker/setup-buildx-action@v3
    
    - name: Log in to Container Registry
      uses: docker/login-action@v3
      with:
        registry: ${{ env.REGISTRY }}
        username: ${{ github.actor }}
        password: ${{ secrets.GITHUB_TOKEN }}
    
    - name: Extract metadata
      id: meta
      uses: docker/metadata-action@v5
      with:
        images: ${{ env.REGISTRY }}/${{ env.IMAGE_NAME }}/${{ matrix.component }}
        tags: |
          type=ref,event=branch
          type=ref,event=pr
          type=sha,prefix={{branch}}-
          type=raw,value=latest,enable={{is_default_branch}}
    
    - name: Build and push Docker image
      uses: docker/build-push-action@v5
      with:
        context: .
        file: ./docker/Dockerfile.${{ matrix.component }}
        push: true
        tags: ${{ steps.meta.outputs.tags }}
        labels: ${{ steps.meta.outputs.labels }}
        cache-from: type=gha
        cache-to: type=gha,mode=max
        platforms: linux/amd64,linux/arm64

  security-scan:
    needs: build
    runs-on: ubuntu-latest
    steps:
    - name: Run Trivy vulnerability scanner
      uses: aquasecurity/trivy-action@master
      with:
        image-ref: ${{ env.REGISTRY }}/${{ env.IMAGE_NAME }}/collector:latest
        format: 'sarif'
        output: 'trivy-results.sarif'
    
    - name: Upload Trivy scan results
      uses: github/codeql-action/upload-sarif@v2
      with:
        sarif_file: 'trivy-results.sarif'

  deploy-staging:
    needs: [test, build]
    if: github.ref == 'refs/heads/develop'
    runs-on: ubuntu-latest
    environment: staging
    steps:
    - uses: actions/checkout@v4
    
    - name: Configure kubectl
      uses: azure/k8s-set-context@v3
      with:
        method: kubeconfig
        kubeconfig: ${{ secrets.KUBE_CONFIG_STAGING }}
    
    - name: Deploy to staging
      run: |
        kubectl apply -f k8s/namespace.yaml
        kubectl apply -f k8s/staging/
        kubectl rollout status deployment/coinbase-collector -n alphapulse-staging
        kubectl rollout status deployment/websocket-server -n alphapulse-staging
    
    - name: Run integration tests
      run: |
        ./scripts/integration-tests.sh staging

  deploy-production:
    needs: [test, build, deploy-staging]
    if: github.ref == 'refs/heads/main'
    runs-on: ubuntu-latest
    environment: production
    steps:
    - uses: actions/checkout@v4
    
    - name: Configure kubectl
      uses: azure/k8s-set-context@v3
      with:
        method: kubeconfig
        kubeconfig: ${{ secrets.KUBE_CONFIG_PRODUCTION }}
    
    - name: Deploy to production
      run: |
        kubectl apply -f k8s/namespace.yaml
        kubectl apply -f k8s/production/
        
        # Rolling deployment with zero downtime
        kubectl patch deployment coinbase-collector -n alphapulse -p \
          '{"spec":{"template":{"metadata":{"annotations":{"deployment/restart":"'$(date +%s)'"}}}}}'
        kubectl rollout status deployment/coinbase-collector -n alphapulse --timeout=600s
        
        kubectl patch deployment websocket-server -n alphapulse -p \
          '{"spec":{"template":{"metadata":{"annotations":{"deployment/restart":"'$(date +%s)'"}}}}}'
        kubectl rollout status deployment/websocket-server -n alphapulse --timeout=600s
    
    - name: Run smoke tests
      run: |
        ./scripts/smoke-tests.sh production
    
    - name: Notify deployment success
      uses: 8398a7/action-slack@v3
      with:
        status: success
        text: "🚀 AlphaPulse deployed to production successfully!"
      env:
        SLACK_WEBHOOK_URL: ${{ secrets.SLACK_WEBHOOK }}
```

## 🔧 Deployment Scripts

### **Zero-Downtime Deployment Script**

```bash
#!/bin/bash
# scripts/deploy.sh

set -euo pipefail

ENVIRONMENT=${1:-staging}
NAMESPACE="alphapulse-${ENVIRONMENT}"
IMAGE_TAG=${2:-latest}

echo "🚀 Deploying AlphaPulse to ${ENVIRONMENT} environment"

# Function to check deployment status
check_deployment_status() {
    local deployment=$1
    echo "📊 Checking deployment status for ${deployment}"
    
    kubectl rollout status deployment/${deployment} -n ${NAMESPACE} --timeout=600s
    
    # Verify pods are healthy
    local ready_pods=$(kubectl get deployment ${deployment} -n ${NAMESPACE} -o jsonpath='{.status.readyReplicas}')
    local desired_pods=$(kubectl get deployment ${deployment} -n ${NAMESPACE} -o jsonpath='{.spec.replicas}')
    
    if [[ "${ready_pods}" != "${desired_pods}" ]]; then
        echo "❌ Deployment ${deployment} is not healthy: ${ready_pods}/${desired_pods} pods ready"
        exit 1
    fi
    
    echo "✅ Deployment ${deployment} is healthy: ${ready_pods}/${desired_pods} pods ready"
}

# Function to run health checks
run_health_checks() {
    echo "🩺 Running health checks"
    
    # Check collector health
    local collector_pod=$(kubectl get pods -n ${NAMESPACE} -l app=alphapulse-collector -o jsonpath='{.items[0].metadata.name}')
    kubectl exec ${collector_pod} -n ${NAMESPACE} -- curl -f http://localhost:8080/health
    
    # Check WebSocket server health
    local websocket_pod=$(kubectl get pods -n ${NAMESPACE} -l app=alphapulse-websocket-server -o jsonpath='{.items[0].metadata.name}')
    kubectl exec ${websocket_pod} -n ${NAMESPACE} -- curl -f http://localhost:8766/health
    
    echo "✅ All health checks passed"
}

# Function to run performance validation
validate_performance() {
    echo "⚡ Validating performance metrics"
    
    # Check latency metrics
    local metrics_endpoint="http://$(kubectl get svc coinbase-collector-service -n ${NAMESPACE} -o jsonpath='{.spec.clusterIP}'):8080/metrics"
    
    # Use a temporary pod to fetch metrics
    kubectl run temp-metrics-checker --rm -i --restart=Never --image=curlimages/curl -- \
        curl -s ${metrics_endpoint} | grep "latency_microseconds" || true
    
    echo "✅ Performance validation completed"
}

# Main deployment flow
main() {
    # Apply namespace and RBAC
    kubectl apply -f k8s/namespace.yaml
    kubectl apply -f k8s/rbac.yaml
    
    # Apply shared memory volume
    kubectl apply -f k8s/shared-memory-volume.yaml
    
    # Deploy collectors with rolling update
    echo "🔄 Deploying collectors"
    kubectl set image deployment/coinbase-collector collector=alphapulse/collector:${IMAGE_TAG} -n ${NAMESPACE}
    kubectl set image deployment/kraken-collector collector=alphapulse/collector:${IMAGE_TAG} -n ${NAMESPACE}
    kubectl set image deployment/binance-collector collector=alphapulse/collector:${IMAGE_TAG} -n ${NAMESPACE}
    
    # Wait for collectors to be ready
    check_deployment_status "coinbase-collector"
    check_deployment_status "kraken-collector"
    check_deployment_status "binance-collector"
    
    # Deploy WebSocket server
    echo "🔄 Deploying WebSocket server"
    kubectl set image deployment/websocket-server websocket-server=alphapulse/websocket-server:${IMAGE_TAG} -n ${NAMESPACE}
    check_deployment_status "websocket-server"
    
    # Deploy Python bindings service
    echo "🔄 Deploying Python bindings service"
    kubectl set image deployment/python-bindings-service python-bindings=alphapulse/python-bindings:${IMAGE_TAG} -n ${NAMESPACE}
    check_deployment_status "python-bindings-service"
    
    # Run validation
    run_health_checks
    validate_performance
    
    echo "🎉 Deployment to ${ENVIRONMENT} completed successfully!"
}

# Execute main function
main "$@"
```

### **Rollback Script**

```bash
#!/bin/bash
# scripts/rollback.sh

set -euo pipefail

ENVIRONMENT=${1:-staging}
NAMESPACE="alphapulse-${ENVIRONMENT}"
REVISION=${2:-}

echo "🔄 Rolling back AlphaPulse in ${ENVIRONMENT} environment"

rollback_deployment() {
    local deployment=$1
    
    if [[ -n "${REVISION}" ]]; then
        echo "Rolling back ${deployment} to revision ${REVISION}"
        kubectl rollout undo deployment/${deployment} --to-revision=${REVISION} -n ${NAMESPACE}
    else
        echo "Rolling back ${deployment} to previous revision"
        kubectl rollout undo deployment/${deployment} -n ${NAMESPACE}
    fi
    
    kubectl rollout status deployment/${deployment} -n ${NAMESPACE} --timeout=300s
}

# Rollback all deployments
rollback_deployment "coinbase-collector"
rollback_deployment "kraken-collector"
rollback_deployment "binance-collector"
rollback_deployment "websocket-server"
rollback_deployment "python-bindings-service"

echo "✅ Rollback completed successfully"
```

## 📊 Monitoring and Observability

### **Prometheus Configuration**

```yaml
# monitoring/prometheus.yml
global:
  scrape_interval: 15s
  evaluation_interval: 15s

rule_files:
  - "alert_rules.yml"

scrape_configs:
  - job_name: 'alphapulse-collectors'
    kubernetes_sd_configs:
    - role: pod
      namespaces:
        names:
        - alphapulse
    relabel_configs:
    - source_labels: [__meta_kubernetes_pod_annotation_prometheus_io_scrape]
      action: keep
      regex: true
    - source_labels: [__meta_kubernetes_pod_annotation_prometheus_io_path]
      action: replace
      target_label: __metrics_path__
      regex: (.+)
    - source_labels: [__address__, __meta_kubernetes_pod_annotation_prometheus_io_port]
      action: replace
      regex: ([^:]+)(?::\d+)?;(\d+)
      replacement: $1:$2
      target_label: __address__
    - action: labelmap
      regex: __meta_kubernetes_pod_label_(.+)
    - source_labels: [__meta_kubernetes_namespace]
      action: replace
      target_label: kubernetes_namespace
    - source_labels: [__meta_kubernetes_pod_name]
      action: replace
      target_label: kubernetes_pod_name

  - job_name: 'alphapulse-websocket-server'
    static_configs:
    - targets: ['websocket-server-service.alphapulse.svc.cluster.local:8766']

alerting:
  alertmanagers:
  - static_configs:
    - targets:
      - alertmanager.monitoring.svc.cluster.local:9093
```

### **Alert Rules**

```yaml
# monitoring/alert_rules.yml
groups:
- name: alphapulse_alerts
  rules:
  - alert: HighLatency
    expr: alphapulse_shared_memory_latency_microseconds > 50
    for: 30s
    labels:
      severity: warning
    annotations:
      summary: "High shared memory latency detected"
      description: "Shared memory latency is {{ $value }}μs, above 50μs threshold"

  - alert: CollectorDown
    expr: up{job="alphapulse-collectors"} == 0
    for: 30s
    labels:
      severity: critical
    annotations:
      summary: "Collector is down"
      description: "AlphaPulse collector {{ $labels.instance }} is down"

  - alert: LowCompressionRatio
    expr: alphapulse_delta_compression_ratio < 0.99
    for: 60s
    labels:
      severity: warning
    annotations:
      summary: "Low delta compression ratio"
      description: "Delta compression ratio is {{ $value }}, below 99% threshold"

  - alert: WebSocketServerDown
    expr: up{job="alphapulse-websocket-server"} == 0
    for: 30s
    labels:
      severity: critical
    annotations:
      summary: "WebSocket server is down"
      description: "AlphaPulse WebSocket server is unreachable"

  - alert: HighMemoryUsage
    expr: container_memory_usage_bytes{pod=~".*alphapulse.*"} / container_spec_memory_limit_bytes > 0.9
    for: 60s
    labels:
      severity: warning
    annotations:
      summary: "High memory usage"
      description: "Pod {{ $labels.pod }} memory usage is above 90%"
```

## 🔐 Security Configuration

### **Network Policies**

```yaml
# k8s/network-policy.yaml
apiVersion: networking.k8s.io/v1
kind: NetworkPolicy
metadata:
  name: alphapulse-network-policy
  namespace: alphapulse
spec:
  podSelector:
    matchLabels:
      app: alphapulse-collector
  policyTypes:
  - Ingress
  - Egress
  ingress:
  - from:
    - namespaceSelector:
        matchLabels:
          name: monitoring
    ports:
    - protocol: TCP
      port: 8080
  egress:
  - {} # Allow all egress for external exchange connections
  
---
apiVersion: networking.k8s.io/v1
kind: NetworkPolicy
metadata:
  name: websocket-server-network-policy
  namespace: alphapulse
spec:
  podSelector:
    matchLabels:
      app: alphapulse-websocket-server
  policyTypes:
  - Ingress
  - Egress
  ingress:
  - ports:
    - protocol: TCP
      port: 8765
    - protocol: TCP
      port: 8766
  egress:
  - to:
    - podSelector:
        matchLabels:
          app: alphapulse-collector
```

### **Pod Security Standards**

```yaml
# k8s/pod-security.yaml
apiVersion: v1
kind: Pod
metadata:
  name: alphapulse-collector
  namespace: alphapulse
spec:
  securityContext:
    runAsNonRoot: true
    runAsUser: 1000
    runAsGroup: 1000
    fsGroup: 1000
    seccompProfile:
      type: RuntimeDefault
  containers:
  - name: collector
    securityContext:
      allowPrivilegeEscalation: false
      readOnlyRootFilesystem: true
      capabilities:
        drop:
        - ALL
    resources:
      limits:
        memory: "256Mi"
        cpu: "500m"
      requests:
        memory: "128Mi"
        cpu: "250m"
```

## 🎯 Performance Targets

### **Deployment Metrics**

| Metric | Target | Measurement |
|--------|--------|-------------|
| **Deployment Time** | <5 minutes | Time from trigger to healthy |
| **Zero Downtime** | 100% | No dropped connections during deploy |
| **Rollback Time** | <2 minutes | Time to restore previous version |
| **Health Check Response** | <1 second | Time for /health endpoint |
| **Resource Efficiency** | >80% | CPU and memory utilization |

### **Production SLA**

| Service | Availability | Latency | Throughput |
|---------|-------------|---------|------------|
| **Collectors** | 99.99% | <10μs shared memory | 100k+ msgs/sec |
| **WebSocket Server** | 99.9% | <1ms broadcast | 10k+ concurrent clients |
| **Python Bindings** | 99.9% | <10μs overhead | Real-time analysis |

## 🚨 Disaster Recovery

### **Backup Strategy**

```bash
# scripts/backup.sh
#!/bin/bash

# Backup shared memory state
kubectl exec -n alphapulse deployment/coinbase-collector -- \
  tar czf - /tmp/alphapulse_shm | \
  aws s3 cp - s3://alphapulse-backups/shared-memory/$(date +%Y%m%d-%H%M%S).tar.gz

# Backup configuration
kubectl get all -n alphapulse -o yaml > backup-$(date +%Y%m%d).yaml
```

### **Recovery Procedures**

1. **Service Recovery**: Automatic pod restart and health checks
2. **Data Recovery**: Restore from S3 shared memory backups
3. **Full Cluster Recovery**: Redeploy from GitOps repository
4. **Cross-Region Failover**: Switch to backup region within 60 seconds

This production deployment pipeline ensures AlphaPulse maintains its ultra-low latency performance while providing enterprise-grade reliability, security, and operational excellence.