
export namespace ClusterTableApi {
  export interface PageFetchParams {
    [key: string]: any;
    page: number;
    page_size: number;
  }
}

import { requestClient } from '#/api/request';
import type { BasicResult } from '#/api/core';

export async function getClusterTableApi(params: ClusterTableApi.PageFetchParams) {
  return requestClient.get('/clusters/', { params });
}

export async function addClusterApi(params: Object) {
  return requestClient.post<BasicResult>('/clusters', params);
}

export async function updateClusterApi(name: string | any, params: Object) {
  return requestClient.put(`/clusters/${name}`, params);
}

export async function deleteClusterApi(name: string) {
  return requestClient.delete(`/clusters/${name}`);
}

export async function getClusterApi(name: string) {
  return requestClient.get(`/clusters/${name}`);
}

export async function getAllClustersApi() {
  return requestClient.get('/clusters/all');
}
