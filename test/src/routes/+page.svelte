<script lang="ts">
    let { data } = $props();
</script>

<div class="flow-root p-4">
    <dl class="-my-3 divide-y divide-gray-200 text-sm *:even:bg-gray-50">
        <div class="grid grid-cols-1 gap-1 p-3 sm:grid-cols-3 sm:gap-4">
            <dt class="font-medium text-gray-900">Request ID</dt>

            <dd class="text-gray-700 sm:col-span-2">
                {data.headers["x-request-id"] ?? "Unknown"}
            </dd>
        </div>

        <div class="grid grid-cols-1 gap-1 p-3 sm:grid-cols-3 sm:gap-4">
            <dt class="font-medium text-gray-900">Client IP</dt>

            <dd class="text-gray-700 sm:col-span-2">
                {data.headers["x-forwarded-for"] ?? "Unknown"}
            </dd>
        </div>

        <div class="grid grid-cols-1 gap-1 p-3 sm:grid-cols-3 sm:gap-4">
            <dt class="font-medium text-gray-900">JA3 Fingerprint</dt>

            <dd class="text-gray-700 sm:col-span-2">
                {data.headers["x-ja3-fingerprint"] ?? "Unknown"}
            </dd>
        </div>

        <div class="grid grid-cols-1 gap-1 p-3 sm:grid-cols-3 sm:gap-4">
            <dt class="font-medium text-gray-900">JA4 Fingerprint</dt>

            <dd class="text-gray-700 sm:col-span-2">
                {data.headers["x-ja4-fingerprint"] ?? "Unknown"}
            </dd>
        </div>

        <div class="grid grid-cols-1 gap-1 p-3 sm:grid-cols-3 sm:gap-4">
            <dt class="font-medium text-gray-900">HTTP2 Fingerprint</dt>

            <dd class="text-gray-700 sm:col-span-2">
                {data.headers["x-http2-fingerprint"] ?? "Unknown"}
            </dd>
        </div>
    </dl>
</div>