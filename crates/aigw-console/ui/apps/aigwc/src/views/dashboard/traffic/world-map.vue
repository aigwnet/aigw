<script setup lang="ts">
import { ref, watch } from 'vue';
import { EchartsUI, useEcharts } from '@vben/plugins/echarts';

import type { EchartsUIType } from '@vben/plugins/echarts';
import { type AnalyticsApi } from '#/api';

import { MapChart } from 'echarts/charts';
import {
    GeoComponent,
    VisualMapComponent,
} from 'echarts/components';
import * as echartsCore from 'echarts/core';

echartsCore.use([
    MapChart,
    GeoComponent,
    VisualMapComponent,
]);

import worldMap from './world.json';
echartsCore.registerMap('world', worldMap as any);


const chartRef = ref<EchartsUIType>();
const { renderEcharts } = useEcharts(chartRef);

const props = defineProps<{
    data: AnalyticsApi.ExtInfo | undefined;
}>();

var itemStyle = {
    borderWidth: 0.5,
    borderColor: 'black'
};

const chartOption = ref<echarts.EChartsOption>(
    {
        tooltip: {
            trigger: 'item',
            formatter: function (params) {
                if (!params.value) {
                    return '';
                }
                return params.name + ' : ' + params.value;
            }
        },
        visualMap: {
            min: 0,
            max: 1000000,
            text: ['High', 'Low'],
            realtime: true,
            calculable: true,
            color: ['orangered', 'yellow', 'lightskyblue']
        },
        series: [
            {
                type: 'map',
                map: 'world',
                roam: true,
                top: 60,
                width: '100%',
                label: {
                    show: false,
                    textBorderColor: '#fff',
                    textBorderWidth: 1,
                },
                itemStyle: itemStyle,
                data: [
                ]
            }
        ]

    });

watch(
    () => props.data,
    (newData) => {
        //if (!newData || newData.length === 0) return;

         const data = Object.entries(newData?.http_country).map(([name, value]) => ({
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
    <div class="chart-wrapper ">
        <EchartsUI ref="chartRef" height="100%" :option="chartOption" />
    </div>
</template>


<style scoped>
.chart-wrapper {
    width: 100%;
    height: 700px;
}
</style>