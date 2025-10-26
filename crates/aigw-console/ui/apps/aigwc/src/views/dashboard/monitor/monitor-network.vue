<script setup lang="ts">
import { ref, watch } from 'vue';
import { EchartsUI, useEcharts } from '@vben/plugins/echarts';
import type { EchartsUIType } from '@vben/plugins/echarts';
import { type AnalyticsApi } from '#/api';
const chartRef = ref<EchartsUIType>();
const { renderEcharts } = useEcharts(chartRef);

const props = defineProps<{
    data: AnalyticsApi.AnalyticsMonitor[];
}>();

const chartOption = ref<echarts.EChartsOption>({
    tooltip: { trigger: 'axis' },
    legend: { data: ['Network Send', 'Network Received'] },
    grid: { left: '3%', right: '4%', bottom: '3%', containLabel: true },
    xAxis: { type: 'category', boundaryGap: false, data: [] },
    yAxis: {
        type: 'value',
        axisLabel: {
            formatter: '{value} Kb/s',
        },
    },
    series: [
        {
            name: 'Network Send',
            type: 'line',
            smooth: true,
            data: [],
        },
        {
            name: 'Network Received',
            type: 'line',
            smooth: true,
            data: [],
        },
    ],
});

watch(
    () => props.data,
    (newData) => {
        //if (!newData || newData.length === 0) return;

        const times = newData.map(item => item.time);
        const net_send = newData.map(item => (item.net_send / 1024).toFixed(2));
        const net_received = newData.map(item => (item.net_received / 1024).toFixed(2));

        chartOption.value = {
            ...chartOption.value,
            xAxis: {
                ...chartOption.value.xAxis,
                data: times,
            },
            series: [
                {
                    ...(chartOption.value.series as any[])?.[0],
                    data: net_send
                },
                {
                    ...(chartOption.value.series as any[])?.[1],
                    data: net_received
                },
            ],
        };

        renderEcharts({ ...chartOption.value } as any);
    },
    { immediate: true }
);
</script>

<template>
    <EchartsUI ref="chartRef" :option="chartOption" autoresize />
</template>