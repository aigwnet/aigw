<script setup lang="ts">
import { ref, watch, h, onMounted, computed } from 'vue'
import { Card, Row, Col, FormItem, Input, Select, InputNumber, Switch, RadioGroup, RadioButton, Textarea, Button, Divider } from 'ant-design-vue'
import { Plus, createIconifyIcon } from '@vben/icons'
import DynamicHeader from './DynamicHeader.vue'
import RemoveHeader from './RemoveHeader.vue'

import { type LocationItem, defaultLocationItem } from '#/types';
const DeleteIcon = createIconifyIcon('ant-design:delete-outlined');

const props = withDefaults(defineProps<{
    min?: number
    max?: number
    namePath: string | string[]
}>(), {
    modelValue: () => [],
    min: 1,
    max: 10,
})

const modelValue = defineModel<LocationItem[]>({
    required: true,
    default: () => []
})

const httpVersionOptions = ref([
    { label: 'HTTP/1.1', value: 'H1' },
    { label: 'HTTP/2', value: 'H2' },
    { label: 'HTTP/2 Over HTTP/1.1', value: 'H2H1' },
]);


const canAdd = computed(() => modelValue.value.length < props.max)
const canRemove = computed(() => modelValue.value.length > props.min)

const addItem = () => {
    if (!canAdd.value)
        return;
    modelValue.value.push(defaultLocationItem());
}

const removeItem = (index: number) => {
    if (!canRemove.value)
        return;
    modelValue.value.splice(index, 1)
}

const ensureMinFields = () => {
    while (modelValue.value.length < props.min) {
        modelValue.value.push(defaultLocationItem());
    }
}

onMounted(() => {
    ensureMinFields()
})
watch(() => props.min, ensureMinFields)

const getFieldPath = (index: number, fieldName: string) => {
    return (Array.isArray(props.namePath)) ? props.namePath.join('_') : props.namePath + "_" + index + "_" + fieldName
}

</script>

<template>

    <div class="w-full">
        <div class="flex justify-end mb-4">
            <Button v-if="canAdd" dashed @click="addItem" class="mb-4 ">
                <Plus /> {{ $t('page.add') }}
            </Button>
        </div>
        <Card v-for="(item, index) in modelValue" :key="index" class="mb-4 p-4">
            <Row>
                <Col :span="24" class="flex justify-between items-center mb-4">
                <h4 class="text-base font-medium text-blue-600">Location {{ index + 1 }}</h4>
                <Button v-if="canRemove" danger shape="circle" :icon="h(DeleteIcon)" size="small"
                    @click="removeItem(index)" />
                </Col>
            </Row>
            <Row>
                <Col :span="24">
                <FormItem :colon="false" :label="$t('page.site.path')" :label-col="{ span: 3 }"
                    :name="getFieldPath(index, 'path')">
                    <Input v-model:value="item.path" placeholder="/" />
                </FormItem>
                </Col>
            </Row>
            <Row>
                <Col :span="24">
                <FormItem :colon="false" :label="$t('page.site.proxy')" :label-col="{ span: 3 }"
                    :name="getFieldPath(index, 'proxy')">
                    <Switch v-model:checked="item.proxy" />
                </FormItem>
                </Col>
            </Row>

            <Row v-show="item.proxy">
                <Col :span="24">
                <FormItem :colon="false" :label="$t('page.site.protocol')" :label-col="{ span: 3 }"
                    :name="getFieldPath(index, 'protocol')">
                    <RadioGroup :id="`protocol-${index}`" :name="`protocol-${index}`" v-model:value="item.protocol">
                        <RadioButton value="http">http</RadioButton>
                        <RadioButton value="https">https</RadioButton>
                    </RadioGroup>
                </FormItem>
                </Col>
            </Row>
            <Row v-show="item.proxy">
                <Col :span="24">
                <FormItem :colon="false" :label="$t('page.site.connectionTimeout')" :label-col="{ span: 3 }"
                    :name="getFieldPath(index, 'connection_timeout')">
                    <InputNumber v-model:value="item.connection_timeout" placeholder="5">
                        <template #addonAfter>
                            s
                        </template>
                    </InputNumber>
                </FormItem>
                </Col>
            </Row>
            <Row v-show="item.proxy">
                <Col :span="24">
                <FormItem :colon="false" :label="$t('page.site.readTimeout')" :label-col="{ span: 3 }"
                    :name="getFieldPath(index, 'read_timeout')">
                    <InputNumber v-model:value="item.read_timeout" placeholder="5">
                        <template #addonAfter>
                            s
                        </template>
                    </InputNumber>
                </FormItem>
                </Col>
            </Row>
            <Row v-show="item.proxy">
                <Col :span="24">
                <FormItem :colon="false" :label="$t('page.site.writeTimeout')" :label-col="{ span: 3 }"
                    :name="getFieldPath(index, 'write_timeout')">
                    <InputNumber v-model:value="item.write_timeout" placeholder="5">
                        <template #addonAfter>
                            s
                        </template>
                    </InputNumber>
                </FormItem>
                </Col>
            </Row>
            <Row v-show="item.proxy">
                <Col :span="24">
                <FormItem :colon="false" :label="$t('page.site.idleTimeout')" :label-col="{ span: 3 }"
                    :name="getFieldPath(index, 'idle_timeout')">
                    <InputNumber v-model:value="item.idle_timeout" placeholder="30">
                        <template #addonAfter>
                            s
                        </template>
                    </InputNumber>
                </FormItem>
                </Col>
            </Row>
            <Row v-show="item.proxy">
                <Col :span="24">
                <FormItem :colon="false" :label="$t('page.site.sni')" :label-col="{ span: 3 }"
                    :name="getFieldPath(index, 'sni')">
                    <Input v-model:value="item.sni" placeholder="" />
                </FormItem>
                </Col>
            </Row>
            <Row v-show="item.proxy">
                <Col :span="24">
                <FormItem :colon="false" :label="$t('page.site.cmbs')" :label-col="{ span: 3 }"
                    :name="getFieldPath(index, 'client_max_body_size')" :extra="$t('page.site.cmbsTip')">
                    <InputNumber v-model:value="item.client_max_body_size" placeholder="0">
                        <template #addonAfter>
                            B
                        </template>
                    </InputNumber>
                </FormItem>
                </Col>
            </Row>
            <Row v-show="item.proxy">
                <Col :span="24">
                <FormItem :colon="false" :label="$t('page.site.rewrite')" :label-col="{ span: 3 }"
                    :name="getFieldPath(index, 'rewrite')">
                    <Input v-model:value="item.rewrite" placeholder="Example: ^/(.*) /v2/api/$1" />
                </FormItem>
                </Col>
            </Row>
            <Row v-show="item.proxy">
                <Col :span="24">
                <FormItem :colon="false" :label="$t('page.site.httpVersion')" :label-col="{ span: 3 }"
                    :name="getFieldPath(index, 'http_version')">
                    <Select v-model:value="item.http_version" :options="httpVersionOptions" />
                </FormItem>
                </Col>
            </Row>
            <Row v-show="item.proxy">
                <Col :span="24">
                <FormItem :colon="false" :label="$t('page.site.upstream')" :label-col="{ span: 3 }"
                    :name="getFieldPath(index, 'upstream')" :extra="$t('page.site.upstreamTip')">
                    <Textarea v-model:value="item.upstream" :rows="5" placeholder="Enter something..." />
                </FormItem>
                </Col>
            </Row>
            <Row v-show="!item.proxy">
                <Col :span="24">
                <FormItem :colon="false" :label="$t('page.site.rootDir')" :label-col="{ span: 3 }"
                    :name="getFieldPath(index, 'root_dir')">
                    <Input v-model:value="item.root_dir" placeholder="" />
                </FormItem>
                </Col>
            </Row>
            <Row v-show="!item.proxy">
                <Col :span="24">
                <FormItem :colon="false" :label="$t('page.site.autoIndex')" :label-col="{ span: 3 }"
                    :name="getFieldPath(index, 'auto_index')">
                    <Switch v-model:checked="item.auto_index" />
                </FormItem>
                </Col>
            </Row>

            <Divider v-show="item.proxy" />
            <Row v-show="item.proxy">
                <Col :span="24">
                <DynamicHeader :label="$t('page.site.addProxyHeaders')" :min="1" :max="10"
                    :namePath="getFieldPath(index, 'proxy_add_headers')" v-model:modelValue="item.proxy_add_headers" />
                </Col>
            </Row>

            <Divider v-show="item.proxy" />
            <Row v-show="item.proxy">
                <Col :span="24">
                <DynamicHeader :label="$t('page.site.setProxyHeaders')" :min="1" :max="10"
                    :namePath="getFieldPath(index, 'proxy_set_headers')" v-model:modelValue="item.proxy_set_headers" />
                </Col>
            </Row>

            <Divider v-show="item.proxy" />
            <Row v-show="item.proxy">
                <Col :span="24">
                <RemoveHeader :label="$t('page.site.removeProxyHeaders')" :min="1" :max="10"
                    :namePath="getFieldPath(index, 'proxy_remove_headers')"
                    v-model:modelValue="item.proxy_remove_headers" />
                </Col>
            </Row>
            <Divider />
            <Row>
                <Col :span="24">
                <DynamicHeader :label="$t('page.site.addResponseHeaders')" :min="1" :max="10"
                    :namePath="getFieldPath(index, 'response_add_headers')"
                    v-model:modelValue="item.response_add_headers" />
                </Col>
            </Row>

            <Divider />
            <Row>
                <Col :span="24">
                <DynamicHeader :label="$t('page.site.setResponseHeaders')" :min="1" :max="10"
                    :namePath="getFieldPath(index, 'response_set_headers')"
                    v-model:modelValue="item.response_set_headers" />
                </Col>
            </Row>

            <Divider />
            <Row>
                <Col :span="24">
                <RemoveHeader :label="$t('page.site.removeResponseHeaders')" :min="1" :max="10"
                    :namePath="getFieldPath(index, 'response_remove_headers')"
                    v-model:modelValue="item.response_remove_headers" />
                </Col>
            </Row>

        </Card>

    </div>

</template>
