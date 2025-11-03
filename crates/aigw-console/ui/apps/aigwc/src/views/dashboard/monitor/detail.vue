<script lang="ts" setup>
import { ref, computed } from 'vue';
import {
    Page, AnalysisChartCard
} from '@vben/common-ui';
import { getAnalyticsMonitorServerApi, type AnalyticsApi } from '#/api';
import { $t } from '#/locales';
import { useRoute } from 'vue-router';
import { useTabs } from '@vben/hooks';

import MonitorCpu from './monitor-cpu.vue';
import MonitorCpuLoad from './monitor-cpu-load.vue';
import MonitorMem from './monitor-mem.vue';
import MonitorDisk from './monitor-disk.vue';
import MonitorIO from './monitor-io.vue';
import MonitorNet from './monitor-network.vue';
import MonitorRt from './monitor-rt.vue';
import MonitorError from './monitor-error.vue';

const route = useRoute();


const { setTabTitle } = useTabs();

const ip = computed(() => {
    return route.params?.ip;
});
const cluster = computed(() => {
    return route.params?.cluster;
});

setTabTitle(`${ip.value} - ` + $t('page.details'));

const analyticsMonitor = ref<Array<AnalyticsApi.AnalyticsMonitor>>([]);

async function loadAnalyticsMonitor() {
    const items = await getAnalyticsMonitorServerApi(cluster.value as string, ip.value as string);
    analyticsMonitor.value = items;
};
loadAnalyticsMonitor();
</script>

<template>

    <Page auto-content-height content-class="flex flex-col gap-4" :title="$t('page.dashboard.monitor')">
        <template #description>
            <div class="text-muted-foreground">
                <p>
                    {{ $t('page.dashboard.monitorServerTip') }}
                </p>
            </div>
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

    </Page>

</template>
