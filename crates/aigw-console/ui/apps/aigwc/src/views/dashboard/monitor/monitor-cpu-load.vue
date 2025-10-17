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
    legend: { data: ['Load1', 'Load5', 'Load15'] },
    grid: { left: '3%', right: '4%', bottom: '3%', containLabel: true },
    xAxis: { type: 'category', boundaryGap: false, data: [] },
    yAxis: {
        type: 'value',
        axisLabel: {
            formatter: '{value}',
        },
    },
    series: [
        {
            name: 'Load1',
            type: 'line',
            smooth: true,
            data: [],
        },
        {
            name: 'Load5',
            type: 'line',
            smooth: true,
            data: [],
        },
        {
            name: 'Load15',
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
        const cpu_load_one = newData.map(item => item.cpu_load_one.toFixed(2));
        const cpu_load_five = newData.map(item => item.cpu_load_five.toFixed(2));
        const cpu_load_fifteen = newData.map(item => item.cpu_load_fifteen.toFixed(2));

        chartOption.value = {
            ...chartOption.value,
            xAxis: {
                ...chartOption.value.xAxis,
                data: times,
            },
            series: [
                {
                    ...(chartOption.value.series as any[])?.[0],
                    data: cpu_load_one
                },
                {
                    ...(chartOption.value.series as any[])?.[1],
                    data: cpu_load_five
                },
                {
                    ...(chartOption.value.series as any[])?.[2],
                    data: cpu_load_fifteen
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