export namespace IpTableApi {
    export interface PageFetchParams {
        [key: string]: any;
        page: number;
        page_size: number;
    }
}
import type { BasicResult } from '#/api/core';
import { requestClient } from '#/api/request';

export async function getClusterIpTableApi(cluster: string | null, type: string, params: IpTableApi.PageFetchParams) {
    return requestClient.get(`/security/ip/${cluster}/${type}`, { params });
}

export async function addClusterIpApi(params: Object) {
    return requestClient.post<BasicResult>('/security/ip/', params);
}