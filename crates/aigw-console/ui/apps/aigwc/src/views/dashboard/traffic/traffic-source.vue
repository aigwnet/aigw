<script setup lang="ts">
import { ref, watch } from 'vue';
import { EchartsUI, useEcharts } from '@vben/plugins/echarts';
import type { EchartsUIType } from '@vben/plugins/echarts';
import { type AnalyticsApi } from '#/api';
const chartRef = ref<EchartsUIType>();
const { renderEcharts } = useEcharts(chartRef);

const props = defineProps<{
  data: AnalyticsApi.ExtInfo | undefined;
}>();

const chartOption = ref<echarts.EChartsOption>({
  tooltip: {
    trigger: 'item',
    formatter: '{b} : {c} ({d}%)'
  },
  legend: {
    top: '5%',
    left: 'center'
  },
  series: [
    {
      type: 'pie',
      radius: '65%',
      center: ['50%', '50%'],
      selectedMode: 'single',
      data: [
      ],
      emphasis: {
        itemStyle: {
          shadowBlur: 10,
          shadowOffsetX: 0,
          shadowColor: 'rgba(0, 0, 0, 0.5)'
        }
      }
    }
  ]
});

watch(
  () => props.data,
  (newData) => {
    //if (!newData || newData.length === 0) return;

    const data = Object.entries(newData?.http_source as any).map(([name, value]) => ({
      name,
      value
    }));

    chartOption.value = {
      ...chartOption.value,
      series: [
        {
          ...(chartOption.value.series as any[])?.[0],
          data: data
        }
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