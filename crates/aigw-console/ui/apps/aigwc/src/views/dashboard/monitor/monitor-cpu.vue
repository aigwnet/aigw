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
    legend: { data: ['Total', 'Current Process'] },
    grid: { left: '3%', right: '4%', bottom: '3%', containLabel: true },
    xAxis: { type: 'category', boundaryGap: false, data: [] },
    yAxis: {
        type: 'value',
        axisLabel: {
            formatter: '{value}%',
        },
    },
    series: [
        {
            name: 'Total',
            type: 'line',
            smooth: true,
            data: [],
        },
        {
            name: 'Current Process',
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
        const cpu = newData.map(item => item.cpu.toFixed(2));
        const cpuCurrent = newData.map(item => item.cpu_current_process.toFixed(2));

        chartOption.value = {
            ...chartOption.value,
            xAxis: {
                ...chartOption.value.xAxis,
                data: times,
            },
            series: [
                {
                    ...(chartOption.value.series as any[])?.[0],
                    data: cpu
                },
                {
                    ...(chartOption.value.series as any[])?.[1],
                    data: cpuCurrent
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