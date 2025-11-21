<script lang="ts" setup>
import { message, Card } from 'ant-design-vue';
import { ref, watch } from 'vue';
import { Page, Loading } from '@vben/common-ui';
import { $t } from '#/locales';
import { getAllClustersApi, addClusterIpApi } from '#/api';
import { useRoute } from 'vue-router';
import { useVbenForm } from '#/adapter/form';
import { clusterStore } from '#/store';
import { ipListRule } from '#/utils'
let clusterAccess = clusterStore();


const LAYER4_OPTIONS = [
    { label: $t('page.security.cluster.ipWhiteList'), value: 1 },
    { label: $t('page.security.cluster.ipBlockList'), value: 2 },
];
const route = useRoute();
const defaultType = Number(route.query.type) || 1;
console.log(defaultType)

const [Form, formApi] = useVbenForm({
    handleSubmit: onSubmit,
    schema: [
        {
            component: 'ApiSelect',
            componentProps: {
                afterFetch: (data: { name: string; }[]) => {
                    return data.map((item: any) => ({
                        label: item.name,
                        value: item.name,
                    }));
                },
                api: getAllClustersApi,
                autoSelect: 'first',
            },
            fieldName: 'cluster_name',
            label: $t('page.cluster.name'),
            defaultValue: clusterAccess.current || undefined,
        },
        {
            component: 'RadioGroup',
            componentProps: {
                options: LAYER4_OPTIONS,
                optionType: 'button',
                buttonStyle: 'solid',
                size: 'default',
            },
            defaultValue: defaultType,
            fieldName: 'type',
            label: $t('page.security.cluster.listType'),
        },
        {
            component: 'Textarea',
            componentProps: {
                placeholder: 'e.g., 2001:db8::/32',
                rows: 20,
                class: 'font-mono',
            },
            fieldName: 'list',
            label: $t('page.security.cluster.ipList'),
            rules: ipListRule,
        },
        {
            component: 'DatePicker',
            fieldName: 'start_time',
            label: $t('page.security.startTime'),
            componentProps: {
                showTime: true,
                format: 'YYYY-MM-DD HH:mm:ss',
                valueFormat: 'YYYY-MM-DD HH:mm:ss',
            },
        },
        {
            component: 'DatePicker',
            fieldName: 'end_time',
            label: $t('page.security.endTime'),
            componentProps: {
                showTime: true,
                format: 'YYYY-MM-DD HH:mm:ss',
                valueFormat: 'YYYY-MM-DD HH:mm:ss',
            },
        },
    ],
    wrapperClass: 'grid-cols-1',
    commonConfig: {
        labelWidth: 200
    },
});

const submitting = ref(false);


async function handleAsyncSubmit(values: Record<string, any>) {

    try {
        var list = Array();
        for (const line of values.list.split(/\r?\n/)) {
            var ip = line;
            var prefix_len;
            if (ip.includes("/")) {
                var tmp = ip.split('/');
                ip = tmp[0];
                prefix_len = parseInt(tmp[1], 10);
            } else if (ip.includes(".")) {
                prefix_len = 32
            } else {
                prefix_len = 128
            }
            list.push({
                "ip": ip,
                "prefix_len": prefix_len
            })
        }

        const processedValues = {
            ...values,
            list: list
        };
        await addClusterIpApi(processedValues);
        formApi.resetForm();
        message.success({
            content: $t('page.security.cluster.addSuccess'),
        });
    } catch {

    } finally {
        submitting.value = false;
    }
}
function onSubmit(values: Record<string, any>) {
    handleAsyncSubmit(values).catch(error => {
        console.error('Submit error:', error);
    });

}


watch(
    () => clusterAccess.current,
    (newCluster, oldCluster) => {
        if (newCluster !== oldCluster) {
            if (oldCluster !== undefined || newCluster !== null) {
                formApi.setFieldValue("cluster", newCluster);
            }
        }
    }
);
</script>

<template>
    <Page content-class="flex flex-col gap-4" description="" title="">
        <Card :title="$t('page.security.cluster.newIp')">
            <Form />

            <div v-if="submitting" class="absolute inset-0 flex items-center justify-center bg-white bg-opacity-30">
                <Loading :spinning="submitting" />
            </div>
        </Card>
    </Page>
</template>