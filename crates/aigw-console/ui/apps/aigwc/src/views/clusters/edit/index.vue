<script lang="ts" setup>
import { computed, onMounted, ref } from 'vue';
import { useRoute } from 'vue-router';
import { $t } from '#/locales';

import { Page, Loading } from '@vben/common-ui';
import { useTabs } from '@vben/hooks';
import { getClusterApi, updateClusterApi } from '#/api';
import { useVbenForm, z } from '#/adapter/form';
import { message, Card } from 'ant-design-vue';
const LAYER4_OPTIONS = [
    { label: $t('page.cluster.secLayer4Wihte'), value: '1' },
    { label: $t('page.cluster.secLayer4Block'), value: '2' },
    { label: $t('page.cluster.secLayer4Disable'), value: '3' },
];
const route = useRoute();

const { setTabTitle } = useTabs();

const index = computed(() => {
    return route.params?.id ?? -1;
});

setTabTitle(`${index.value} - ` + $t('page.details'));

const submitting = ref(true);

const [Form, formApi] = useVbenForm({
    handleSubmit: onSubmit,
    schema: [
        {
            component: 'Input',
            componentProps: {
                placeholder: 'Unique name',
            },
            fieldName: 'name',
            label: $t('page.cluster.name'),
            rules: z.string().min(3, { message: 'Enter at least 3 letters' }),
        },
        {
            component: 'VbenInputPassword',
            componentProps: {
                placeholder: 'Security key',
            },
            fieldName: 'security_key',
            label: $t('page.cluster.key'),
            rules: z.string().min(3, { message: 'Enter at least 3 letters' }),
        },
        {
            component: 'Switch',
            defaultValue: false,
            fieldName: 'enable',
            label: $t('page.cluster.enable'),
        },
        {
            component: 'Switch',
            defaultValue: false,
            fieldName: 'enable_default_site',
            label: $t('page.cluster.enableDefaultSite'),
        },
        {
            component: 'RadioGroup',
            componentProps: {
                options: LAYER4_OPTIONS,
                optionType: 'button',
                buttonStyle: 'solid',
                size: 'default',
            },
            defaultValue: '3',
            fieldName: 'namelist',
            label: $t('page.cluster.secLayer4'),
        },
        {
            component: 'Input',
            componentProps: {
                placeholder: '',
            },
            defaultValue: '',
            fieldName: 'description',
            label: $t('page.cluster.description'),
        },
    ],
    wrapperClass: 'grid-cols-1',
    commonConfig: {
        labelWidth: 200
    },
});


const fetchData = async () => {
    const cluster = await getClusterApi(`${index.value}`);
    if (cluster.enable_white_list == true) {
        cluster.namelist = "1";
    } else if (cluster.enable_block_list == true) {
        cluster.namelist = "2";
    } else {
        cluster.namelist = "3";
    }
    formApi.setValues(cluster);
    submitting.value = false;
}

async function handleAsyncSubmit(values: Record<string, any>) {
    try {
        const processedValues = {
            ...values,
            enable_white_list: values.namelist == "1",
            enable_block_list: values.namelist == "2",
        };
        submitting.value = true;
        await updateClusterApi(index.value, processedValues);
        message.success({
            content: $t('page.cluster.updateSuccess'),
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


onMounted(() => {
    fetchData()
})

</script>

<template>
    <Page content-class="flex flex-col gap-4" description="" title="">
        <Card :title="$t('page.cluster.edit')">
            <Form />

            <div v-if="submitting" class="absolute inset-0 flex items-center justify-center bg-white bg-opacity-30">
                <Loading :spinning="submitting" />
            </div>
        </Card>
    </Page>
</template>