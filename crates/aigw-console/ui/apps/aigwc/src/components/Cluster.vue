<script setup lang="ts">
import { VbenDropdownRadioMenu, VbenIconButton, type VbenDropdownMenuItem } from '@vben-core/shadcn-ui';

import { getAllClustersApi } from '#/api';
import { ref, onMounted } from 'vue';
import { clusterStore } from '#/store';

const CLUSTERS = ref<VbenDropdownMenuItem[]>([]);

const cluster = clusterStore();

onMounted(async () => {
    let data = await getAllClustersApi();
    let value = data.map((item: any) => ({
        label: item.name,
        value: item.name,
    }));

    if (value.length > 0 && !cluster.current) {
        cluster.store(value[0].value);
    }
    CLUSTERS.value = value;
});

import {
    createIconifyIcon,
} from '@vben/icons';
const LicideServer = createIconifyIcon('lucide:server');


defineOptions({
    name: 'Cluster',
});

async function handleUpdate(value: string | undefined) {
    if (!value) return;
    cluster.store(value);
}
</script>

<template>
    <div>
        <VbenDropdownRadioMenu :menus="CLUSTERS" :model-value="cluster.current"
            @update:model-value="handleUpdate">
            <VbenIconButton>
                <LicideServer class="text-foreground size-4" />
            </VbenIconButton>
        </VbenDropdownRadioMenu>
    </div>
</template>
