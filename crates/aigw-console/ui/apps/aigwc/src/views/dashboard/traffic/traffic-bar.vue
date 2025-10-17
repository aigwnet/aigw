<script setup lang="ts">
import { ref, watch } from 'vue';
import { EchartsUI, useEcharts } from '@vben/plugins/echarts';
import type { EchartsUIType } from '@vben/plugins/echarts';
import { type AnalyticsApi } from '#/api';
const chartRef = ref<EchartsUIType>();
const { renderEcharts } = useEcharts(chartRef);

const props = defineProps<{
  data: AnalyticsApi.AnalyticsTrafficItem[] | undefined;
}>();

const chartOption = ref<echarts.EChartsOption>({
  tooltip: {
    axisPointer: {
      lineStyle: {
        color: '#019680',
        width: 1,
      },
    },
    trigger: 'axis',
  },
  legend: { data: ['PV', 'TLS Handshake'] },
  grid: {
    bottom: 0,
    containLabel: true,
    left: '1%',
    right: '1%',
  },
  xAxis: {
    axisTick: {
      show: false,
    },
    boundaryGap: true,
    data: [],
    splitLine: {
      lineStyle: {
        type: 'solid',
        width: 1,
      },
      show: true,
    },
    type: 'category',
  },
  yAxis: [
    {
      axisTick: {
        show: false,
      },
      splitArea: {
        show: true,
      },
      splitNumber: 4,
      type: 'value',
    },
  ],
  series: [
    {
      name: 'PV',
      data: [],
      type: 'bar',
      barWidth: 30, 
    },
    {
      name: 'TLS Handshake',
      data: [],
      type: 'bar',
      barWidth: 30, 
    },
  ],
});

watch(
  () => props.data,
  (newData) => {
    //if (!newData || newData.length === 0) return;

    const times = newData?.map(item => item.time);
    const pv = newData?.map(item => item.pv);
    const tls = newData?.map(item => item.tls);

    chartOption.value = {
      ...chartOption.value,
      xAxis: {
        ...chartOption.value.xAxis,
        data: times,
      },
      series: [
        {
          ...(chartOption.value.series as any[])?.[0],
          data: pv
        },
        {
          ...(chartOption.value.series as any[])?.[1],
          data: tls
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