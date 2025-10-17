<script lang="ts" setup>
import { ref } from 'vue';
import {
    Page, AnalysisChartCard, AnalysisChartsTabs
} from '@vben/common-ui';
import { $t } from '#/locales';
import { getAllClustersApi, getAnalyticsTraffic, getAnalyticsTrafficExt, type AnalyticsApi } from '#/api';
import { useVbenForm } from '#/adapter/form';
import type { TabOption } from '@vben/types';
import TrafficWorldMap from './trraffic-world-map.vue';
import TrafficLine from './traffic-line.vue';
import TrafficBar from './traffic-bar.vue';
import TrafficCode from './traffic-code.vue';
import TrafficSource from './traffic-source.vue';

const isTrafficLoaded = ref(false);
const isTrafficExtLoaded = ref(false);

const clusterRef = ref<string | null>(null);

const analyticsTraffic = ref<AnalyticsApi.AnalyticsTraffic>();
const analyticsTrafficExt = ref<AnalyticsApi.ExtInfo>();

async function loadAnalyticsTraffic(cluster: string) {
    const data = await getAnalyticsTraffic(cluster);
    isTrafficLoaded.value = true;
    analyticsTraffic.value = data;
};

async function loadAnalyticsTrafficExt(cluster: string) {
    const data = await getAnalyticsTrafficExt(cluster);
    isTrafficExtLoaded.value = true;
    analyticsTrafficExt.value = data;
};

const [ClusterForm, _formApi] = useVbenForm({
    schema: [
        {
            component: 'ApiSelect',
            componentProps: {
                afterFetch: (data: { name: string; }[]) => {
                    const options = data.map(item => ({ label: item.name, value: item.name }));
                    if (options.length > 0 && !clusterRef.value) {
                        clusterRef.value = options[0]!.value;
                        loadAnalyticsTraffic(clusterRef.value);
                        loadAnalyticsTrafficExt(clusterRef.value);
                    }
                    return options;
                },
                api: getAllClustersApi,
                onChange: (value: string, _prevValue: string) => {
                    clusterRef.value = value;
                    loadAnalyticsTraffic(value);
                    loadAnalyticsTrafficExt(value);
                },
                autoSelect: 'first',
            },
            fieldName: 'cluster',
            label: '',

        },
    ],
    showDefaultActions: false,
});

const chartTabs: TabOption[] = [
    {
        label: $t('page.dashboard.trafficPvM'),
        value: 'trends_m',
    },
    {
        label: $t('page.dashboard.trafficPvH'),
        value: 'trends_h',
    },
    {
        label: $t('page.dashboard.trafficPvD'),
        value: 'visits',
    },
];

</script>

<template>

    <Page auto-content-height content-class="flex flex-col gap-4" :title="$t('page.dashboard.traffic')">
        <template #description>
            <div class="text-muted-foreground">
                <p>
                    {{ $t('page.dashboard.trafficTip') }}
                </p>
            </div>
        </template>
        <template #extra>
            <ClusterForm class="mb-2" />
        </template>

        <div v-if="isTrafficLoaded">
            <AnalysisChartsTabs :tabs="chartTabs">
                <template #trends_m>
                    <TrafficLine :data="analyticsTraffic?.data_latest_30" />
                </template>
                <template #trends_h>
                    <TrafficLine :data="analyticsTraffic?.data_1day" />
                </template>
                <template #visits>
                    <TrafficBar :data="analyticsTraffic?.data_1month" />
                </template>
            </AnalysisChartsTabs>
        </div>
        <div v-if="isTrafficExtLoaded">
            <div class="w-full md:flex">
                <AnalysisChartCard class="mt-5 md:mr-4 md:mt-0 md:w-1/2" :title="$t('page.dashboard.trafficSC')">
                    <TrafficCode :data="analyticsTrafficExt" />
                </AnalysisChartCard>
                <AnalysisChartCard class="mt-5 md:mt-0 md:w-1/2" :title="$t('page.dashboard.trafficSrc')">
                    <TrafficSource :data="analyticsTrafficExt" />
                </AnalysisChartCard>
            </div>
            <div class="w-full md:flex">
                <AnalysisChartCard class="w-full mt-5" :title="$t('page.dashboard.trafficGeo')">
                    <TrafficWorldMap :data="analyticsTrafficExt" />
                </AnalysisChartCard>
            </div>
        </div>

    </Page>

</template>
