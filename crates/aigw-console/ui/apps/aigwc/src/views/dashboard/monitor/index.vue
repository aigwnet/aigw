<script lang="ts" setup>
import { ref, watch, onMounted, onBeforeUnmount } from 'vue';
import {
    Page, AnalysisChartCard
} from '@vben/common-ui';
import { Button } from 'ant-design-vue';
import { getServerTableApi, getAnalyticsMonitorApi, type AnalyticsApi } from '#/api';
import type { VxeGridProps } from '#/adapter/vxe-table';
import { useVbenVxeGrid } from '#/adapter/vxe-table';
import { useRouter } from 'vue-router';
import { $t } from '#/locales';
import { clusterStore } from '#/store';

import MonitorCpu from './monitor-cpu.vue';
import MonitorCpuLoad from './monitor-cpu-load.vue';
import MonitorMem from './monitor-mem.vue';
import MonitorDisk from './monitor-disk.vue';
import MonitorIO from './monitor-io.vue';
import MonitorNet from './monitor-network.vue';
import MonitorRt from './monitor-rt.vue';
import MonitorError from './monitor-error.vue';

const clusterAccess = clusterStore();
const analyticsMonitor = ref<Array<AnalyticsApi.AnalyticsMonitor>>([]);
let refreshInterval: number | null = null;

async function loadAnalyticsMonitor(cluster: string) {
    const items = await getAnalyticsMonitorApi(cluster);
    analyticsMonitor.value = items;
};

async function reloadAllData() {
    const cluster = clusterAccess.current;
    if (!cluster) return;
    await Promise.all([
        loadAnalyticsMonitor(cluster)
    ]);
}

onMounted(() => {
    reloadAllData();

    refreshInterval = window.setInterval(() => {
        reloadAllData();
    }, 30_000);
});
onBeforeUnmount(() => {
    if (refreshInterval !== null) {
        window.clearInterval(refreshInterval);
        refreshInterval = null;
    }
});

watch(
    () => clusterAccess.current,
    (newCluster, oldCluster) => {
        if (newCluster !== oldCluster) {
            if (oldCluster !== undefined || newCluster !== null) {
                reloadAllData();
            }
        }
    }
);

interface RowType {
    ip: string;
    version: string;
    os_name: string;
    os_version: string;
    os_arch: string;
    cpu_name: string;
    cpu_vendor: string;
    cpu_frequency: number;
    cpu_nums: number;
    status: string;
    gmt_modified: string;
}

const gridOptions: VxeGridProps<RowType> = {
    checkboxConfig: {
        highlight: true,
        labelField: 'name',
    },
    columns: [
        { title: 'No', type: 'seq', width: 50 },
        { field: 'ip', sortable: true, title: 'IP' },
        { field: 'version', sortable: true, title: $t('page.dashboard.version') },
        { slots: { default: 'os_info' }, title: $t('page.dashboard.os') },
        { slots: { default: 'cpu_info' }, title: 'CPU' },
        { field: 'cpu_nums', sortable: true, title: $t('page.dashboard.cpuNum') },
        { field: 'gmt_modified', sortable: true, title: $t('page.modifiedTime') },
        { slots: { default: 'action' }, title: $t('page.operation'), width: 160 },
    ],
    exportConfig: {},
    //height: 'auto',
    keepSource: true,
    proxyConfig: {
        enabled: true,
        ajax: {
            query: async ({ page, sort }) => {
                var cluster = clusterAccess.current!;
                let data = await getServerTableApi(cluster, {
                    cluster: cluster,
                    page: page.currentPage,
                    page_size: page.pageSize,
                    sort_by: sort.field,
                    sort_order: sort.order,
                });

                return data;
            },
        },
        sort: true,
    },
    sortConfig: {
        defaultSort: { field: 'ip', order: 'desc' },
        remote: true,
    },
    toolbarConfig: {
        custom: true,
        export: true,
        refresh: true,
        refreshOptions: { code: 'query' },
        zoom: true,
    },
};

const [Grid, gridApi] = useVbenVxeGrid({
    gridOptions,
});

const router = useRouter();

const onClickServer = (row: RowType) => {
    router.push('/dashboard/mointor/' + clusterAccess.current + "/" + row.ip);
};
</script>

<template>

    <Page auto-content-height content-class="flex flex-col gap-4"
        :title="$t('page.cluster.title') + ' (' + clusterAccess.current + ') ' + $t('page.dashboard.monitor')">
        <template #description>
            <div class="text-muted-foreground">
                <p>
                    {{ $t('page.dashboard.monitorTip') }}
                </p>
            </div>
        </template>
        <template #extra>
            <ClusterForm class="mb-2" />
        </template>

        <div class="w-full md:flex">
            <AnalysisChartCard class="mt-5 md:mr-4 md:mt-0 md:w-1/2" title="CPU">
                <MonitorCpu :data="analyticsMonitor" />
            </AnalysisChartCard>
            <AnalysisChartCard class="mt-5 md:mt-0 md:w-1/2" :title="$t('page.dashboard.load')">
                <MonitorCpuLoad :data="analyticsMonitor" />
            </AnalysisChartCard>
        </div>
        <div class="w-full md:flex">
            <AnalysisChartCard class="mt-5 md:mr-4 md:mt-0 md:w-1/2" :title="$t('page.dashboard.memory')">
                <MonitorMem :data="analyticsMonitor" />
            </AnalysisChartCard>
            <AnalysisChartCard class="mt-5 md:mt-0 md:w-1/2" :title="$t('page.dashboard.disk')">
                <MonitorDisk :data="analyticsMonitor" />
            </AnalysisChartCard>
        </div>
        <div class="w-full md:flex">
            <AnalysisChartCard class="mt-5 md:mr-4 md:mt-0 md:w-1/2" :title="$t('page.dashboard.io')">
                <MonitorIO :data="analyticsMonitor" />
            </AnalysisChartCard>
            <AnalysisChartCard class="mt-5 md:mt-0 md:w-1/2" :title="$t('page.dashboard.network')">
                <MonitorNet :data="analyticsMonitor" />
            </AnalysisChartCard>
        </div>
        <div class="w-full md:flex">
            <AnalysisChartCard class="mt-5 md:mr-4 md:mt-0 md:w-1/2" :title="$t('page.dashboard.rt')">
                <MonitorRt :data="analyticsMonitor" />
            </AnalysisChartCard>
            <AnalysisChartCard class="mt-5 md:mt-0 md:w-1/2" :title="$t('page.dashboard.error')">
                <MonitorError :data="analyticsMonitor" />
            </AnalysisChartCard>
        </div>

        <Grid :table-title="$t('page.dashboard.serverList')" table-title-help="">
            <template #toolbar-tools>
                <Button class="mr-2" type="primary" @click="() => gridApi.query()">
                    {{ $t('page.refreshCurrentPage') }}
                </Button>
                <Button type="primary" @click="() => gridApi.reload()">
                    {{ $t('page.refreshAndReturnFirst') }}
                </Button>
            </template>
            <template #os_info="{ row }">
                {{ row.os_name }} - {{ row.os_version }} / {{ row.os_arch }}
            </template>
            <template #cpu_info="{ row }">
                {{ row.cpu_name }}
            </template>
            <template #action="{ row }">
                <Button type="link" @click="onClickServer(row)">{{ $t('page.viewDetails') }}</Button>
            </template>
        </Grid>

    </Page>

</template>
