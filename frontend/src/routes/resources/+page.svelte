<script lang="ts">
	import { onMount } from 'svelte';
	import { writable } from 'svelte/store';

	interface Resource {
		uri: string;
		name: string;
		description?: string;
		mimeType?: string;
		annotations?: {
			size?: number;
			lastModified?: string;
		};
	}

	interface ResourceContent {
		uri: string;
		mimeType?: string;
		text?: string;
		blob?: string;
	}

	let resources = writable<Resource[]>([]);
	let selectedResource = writable<Resource | null>(null);
	let resourceContent = writable<ResourceContent | null>(null);
	let loading = false;
	let error = '';
	let searchQuery = '';
	let selectedMimeType = '';
	let showContentModal = false;

	// Filter resources based on search and MIME type
	$: filteredResources = $resources.filter(resource => {
		const matchesSearch = !searchQuery || 
			resource.name.toLowerCase().includes(searchQuery.toLowerCase()) ||
			resource.uri.toLowerCase().includes(searchQuery.toLowerCase()) ||
			(resource.description && resource.description.toLowerCase().includes(searchQuery.toLowerCase()));
		
		const matchesMimeType = !selectedMimeType || resource.mimeType === selectedMimeType;
		
		return matchesSearch && matchesMimeType;
	});

	// Get unique MIME types for filtering
	$: mimeTypes = [...new Set($resources.map(r => r.mimeType).filter(Boolean))].sort();

	async function loadResources() {
		loading = true;
		error = '';
		try {
			const response = await fetch('/dashboard/api/resources');
			if (!response.ok) {
				throw new Error(`Failed to load resources: ${response.status}`);
			}
			const data = await response.json();
			resources.set(data.resources || []);
		} catch (err) {
			error = err instanceof Error ? err.message : 'Failed to load resources';
			console.error('Error loading resources:', err);
		} finally {
			loading = false;
		}
	}

	async function readResource(resource: Resource) {
		selectedResource.set(resource);
		loading = true;
		error = '';
		try {
			const response = await fetch('/dashboard/api/resources/read', {
				method: 'POST',
				headers: {
					'Content-Type': 'application/json',
				},
				body: JSON.stringify({ uri: resource.uri })
			});
			
			if (!response.ok) {
				throw new Error(`Failed to read resource: ${response.status}`);
			}
			
			const data = await response.json();
			resourceContent.set(data.content);
			showContentModal = true;
		} catch (err) {
			error = err instanceof Error ? err.message : 'Failed to read resource';
			console.error('Error reading resource:', err);
		} finally {
			loading = false;
		}
	}

	function closeContentModal() {
		showContentModal = false;
		selectedResource.set(null);
		resourceContent.set(null);
	}

	function formatFileSize(bytes?: number): string {
		if (!bytes) return 'Unknown';
		const sizes = ['B', 'KB', 'MB', 'GB'];
		const i = Math.floor(Math.log(bytes) / Math.log(1024));
		return `${(bytes / Math.pow(1024, i)).toFixed(1)} ${sizes[i]}`;
	}

	function formatDate(dateStr?: string): string {
		if (!dateStr) return 'Unknown';
		try {
			return new Date(dateStr).toLocaleString();
		} catch {
			return dateStr;
		}
	}

	function downloadResource(content: ResourceContent) {
		if (content.text) {
			// Text content
			const blob = new Blob([content.text], { type: content.mimeType || 'text/plain' });
			const url = URL.createObjectURL(blob);
			const a = document.createElement('a');
			a.href = url;
			a.download = content.uri.split('/').pop() || 'resource.txt';
			document.body.appendChild(a);
			a.click();
			document.body.removeChild(a);
			URL.revokeObjectURL(url);
		} else if (content.blob) {
			// Binary content (base64)
			try {
				const byteCharacters = atob(content.blob);
				const byteNumbers = new Array(byteCharacters.length);
				for (let i = 0; i < byteCharacters.length; i++) {
					byteNumbers[i] = byteCharacters.charCodeAt(i);
				}
				const byteArray = new Uint8Array(byteNumbers);
				const blob = new Blob([byteArray], { type: content.mimeType || 'application/octet-stream' });
				const url = URL.createObjectURL(blob);
				const a = document.createElement('a');
				a.href = url;
				a.download = content.uri.split('/').pop() || 'resource.bin';
				document.body.appendChild(a);
				a.click();
				document.body.removeChild(a);
				URL.revokeObjectURL(url);
			} catch (err) {
				console.error('Error downloading binary resource:', err);
				error = 'Failed to download binary resource';
			}
		}
	}

	onMount(loadResources);
</script>

<svelte:head>
	<title>MCP Resources - MagicTunnel</title>
</svelte:head>

<div class="min-h-screen bg-gray-50 p-6">
	<div class="max-w-7xl mx-auto">
		<!-- Header -->
		<div class="mb-8">
			<div class="flex justify-between items-center">
				<div>
					<div class="mb-2">
						<h1 class="text-4xl font-bold text-primary-700">MCP Resources</h1>
					</div>
					<p class="mt-2 text-gray-600">Browse and access MCP resources from all providers</p>
				</div>
				<button
					on:click={loadResources}
					disabled={loading}
					class="bg-blue-500 hover:bg-blue-700 disabled:bg-gray-400 text-white font-bold py-2 px-4 rounded inline-flex items-center"
				>
					{#if loading}
						<svg class="animate-spin -ml-1 mr-3 h-5 w-5 text-white" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24">
							<circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
							<path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
						</svg>
						Refreshing...
					{:else}
						<svg class="w-4 h-4 mr-2" fill="none" stroke="currentColor" viewBox="0 0 24 24">
							<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" />
						</svg>
						Refresh
					{/if}
				</button>
			</div>
		</div>

		<!-- Error Display -->
		{#if error}
			<div class="mb-6 bg-red-100 border border-red-400 text-red-700 px-4 py-3 rounded relative">
				<strong class="font-bold">Error:</strong>
				<span class="block sm:inline"> {error}</span>
				<button 
					class="absolute top-0 bottom-0 right-0 px-4 py-3"
					on:click={() => error = ''}
				>
					<svg class="fill-current h-6 w-6 text-red-500" role="button" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 20">
						<path d="M14.348 14.849a1.2 1.2 0 0 1-1.697 0L10 11.819l-2.651 3.029a1.2 1.2 0 1 1-1.697-1.697l2.758-3.15-2.759-3.152a1.2 1.2 0 1 1 1.697-1.697L10 8.183l2.651-3.031a1.2 1.2 0 1 1 1.697 1.697l-2.758 3.152 2.758 3.15a1.2 1.2 0 0 1 0 1.698z"/>
					</svg>
				</button>
			</div>
		{/if}

		<!-- Filters -->
		<div class="mb-6 bg-white rounded-lg shadow p-4">
			<div class="grid grid-cols-1 md:grid-cols-2 gap-4">
				<div>
					<label for="search" class="block text-sm font-medium text-gray-700 mb-2">Search Resources</label>
					<input
						id="search"
						type="text"
						bind:value={searchQuery}
						placeholder="Search by name, URI, or description..."
						class="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500"
					/>
				</div>
				<div>
					<label for="mimeType" class="block text-sm font-medium text-gray-700 mb-2">Filter by MIME Type</label>
					<select
						id="mimeType"
						bind:value={selectedMimeType}
						class="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500"
					>
						<option value="">All MIME Types</option>
						{#each mimeTypes as mimeType}
							<option value={mimeType}>{mimeType}</option>
						{/each}
					</select>
				</div>
			</div>
		</div>

		<!-- Stats -->
		<div class="mb-6 grid grid-cols-1 md:grid-cols-3 gap-4">
			<div class="bg-white overflow-hidden shadow rounded-lg">
				<div class="p-5">
					<div class="flex items-center">
						<div class="flex-shrink-0">
							<svg class="h-8 w-8 text-blue-500" fill="none" stroke="currentColor" viewBox="0 0 24 24">
								<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z" />
							</svg>
						</div>
						<div class="ml-5 w-0 flex-1">
							<dl>
								<dt class="text-sm font-medium text-gray-500 truncate">Total Resources</dt>
								<dd class="text-lg font-medium text-gray-900">{$resources.length}</dd>
							</dl>
						</div>
					</div>
				</div>
			</div>

			<div class="bg-white overflow-hidden shadow rounded-lg">
				<div class="p-5">
					<div class="flex items-center">
						<div class="flex-shrink-0">
							<svg class="h-8 w-8 text-green-500" fill="none" stroke="currentColor" viewBox="0 0 24 24">
								<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M3 4a1 1 0 011-1h16a1 1 0 011 1v2.586a1 1 0 01-.293.707l-6.414 6.414a1 1 0 00-.293.707V17l-4 4v-6.586a1 1 0 00-.293-.707L3.293 7.293A1 1 0 013 6.586V4z" />
							</svg>
						</div>
						<div class="ml-5 w-0 flex-1">
							<dl>
								<dt class="text-sm font-medium text-gray-500 truncate">Filtered Results</dt>
								<dd class="text-lg font-medium text-gray-900">{filteredResources.length}</dd>
							</dl>
						</div>
					</div>
				</div>
			</div>

			<div class="bg-white overflow-hidden shadow rounded-lg">
				<div class="p-5">
					<div class="flex items-center">
						<div class="flex-shrink-0">
							<svg class="h-8 w-8 text-purple-500" fill="none" stroke="currentColor" viewBox="0 0 24 24">
								<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M7 21a4 4 0 01-4-4V5a2 2 0 012-2h4a2 2 0 012 2v12a4 4 0 01-4 4zM21 5a2 2 0 00-2-2h-4a2 2 0 00-2 2v12a4 4 0 004 4h4a2 2 0 002-2V5z" />
							</svg>
						</div>
						<div class="ml-5 w-0 flex-1">
							<dl>
								<dt class="text-sm font-medium text-gray-500 truncate">MIME Types</dt>
								<dd class="text-lg font-medium text-gray-900">{mimeTypes.length}</dd>
							</dl>
						</div>
					</div>
				</div>
			</div>
		</div>

		<!-- Resources List -->
		<div class="bg-white shadow overflow-hidden rounded-md">
			<div class="px-4 py-5 sm:p-6">
				<h3 class="text-lg leading-6 font-medium text-gray-900 mb-4">Available Resources</h3>
				
				{#if loading}
					<div class="text-center py-8">
						<svg class="animate-spin mx-auto h-8 w-8 text-blue-500" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24">
							<circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
							<path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
						</svg>
						<p class="mt-2 text-gray-500">Loading resources...</p>
					</div>
				{:else if filteredResources.length === 0}
					<div class="text-center py-8">
						<svg class="mx-auto h-12 w-12 text-gray-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
							<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z" />
						</svg>
						<h3 class="mt-2 text-sm font-medium text-gray-900">No resources found</h3>
						<p class="mt-1 text-sm text-gray-500">
							{searchQuery || selectedMimeType ? 'Try adjusting your filters.' : 'No MCP resources are currently available.'}
						</p>
					</div>
				{:else}
					<div class="space-y-4">
						{#each filteredResources as resource}
							<div class="border border-gray-200 rounded-lg p-4 hover:bg-gray-50 transition-colors">
								<div class="flex justify-between items-start">
									<div class="flex-1">
										<div class="flex items-center space-x-2">
											<h4 class="text-lg font-medium text-gray-900">{resource.name}</h4>
											{#if resource.mimeType}
												<span class="inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium bg-blue-100 text-blue-800">
													{resource.mimeType}
												</span>
											{/if}
										</div>
										<p class="text-sm text-gray-600 mt-1 font-mono">{resource.uri}</p>
										{#if resource.description}
											<p class="text-sm text-gray-700 mt-2">{resource.description}</p>
										{/if}
										{#if resource.annotations}
											<div class="flex space-x-4 mt-2 text-xs text-gray-500">
												{#if resource.annotations.size}
													<span>Size: {formatFileSize(resource.annotations.size)}</span>
												{/if}
												{#if resource.annotations.lastModified}
													<span>Modified: {formatDate(resource.annotations.lastModified)}</span>
												{/if}
											</div>
										{/if}
									</div>
									<div class="ml-4">
										<button
											on:click={() => readResource(resource)}
											disabled={loading}
											class="bg-blue-500 hover:bg-blue-700 disabled:bg-gray-400 text-white font-bold py-2 px-4 rounded text-sm"
										>
											View Content
										</button>
									</div>
								</div>
							</div>
						{/each}
					</div>
				{/if}
			</div>
		</div>

	</div>
</div>

<!-- Resource Content Modal -->
{#if showContentModal && $resourceContent && $selectedResource}
	<div class="fixed inset-0 bg-gray-600 bg-opacity-50 overflow-y-auto h-full w-full z-50">
		<div class="relative top-20 mx-auto p-5 border w-11/12 md:w-3/4 lg:w-1/2 shadow-lg rounded-md bg-white">
			<div class="mt-3">
				<!-- Modal Header -->
				<div class="flex justify-between items-center pb-3 border-b">
					<h3 class="text-lg font-medium text-gray-900">Resource Content</h3>
					<button
						on:click={closeContentModal}
						class="text-gray-400 hover:text-gray-600"
					>
						<svg class="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
							<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
						</svg>
					</button>
				</div>

				<!-- Resource Info -->
				<div class="mt-4">
					<div class="bg-gray-50 p-3 rounded-lg">
						<p class="text-sm"><strong>Name:</strong> {$selectedResource.name}</p>
						<p class="text-sm"><strong>URI:</strong> <span class="font-mono text-xs">{$selectedResource.uri}</span></p>
						{#if $resourceContent.mimeType}
							<p class="text-sm"><strong>MIME Type:</strong> {$resourceContent.mimeType}</p>
						{/if}
					</div>
				</div>

				<!-- Content Display -->
				<div class="mt-4">
					{#if $resourceContent.text}
						<div class="bg-gray-100 p-4 rounded-lg max-h-96 overflow-y-auto">
							<pre class="text-sm whitespace-pre-wrap font-mono">{$resourceContent.text}</pre>
						</div>
					{:else if $resourceContent.blob}
						<div class="bg-gray-100 p-4 rounded-lg text-center">
							<svg class="mx-auto h-12 w-12 text-gray-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
								<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M7 21h10a2 2 0 002-2V9.414a1 1 0 00-.293-.707l-5.414-5.414A1 1 0 0012.586 3H7a2 2 0 00-2 2v14a2 2 0 002 2z" />
							</svg>
							<p class="text-gray-600 mt-2">Binary content ({formatFileSize($resourceContent.blob.length)})</p>
							<p class="text-sm text-gray-500">Use the download button to save this file</p>
						</div>
					{/if}
				</div>

				<!-- Modal Actions -->
				<div class="flex justify-end space-x-3 mt-6 pt-3 border-t">
					<button
						on:click={() => downloadResource($resourceContent)}
						class="bg-green-500 hover:bg-green-700 text-white font-bold py-2 px-4 rounded text-sm"
					>
						Download
					</button>
					<button
						on:click={closeContentModal}
						class="bg-gray-500 hover:bg-gray-700 text-white font-bold py-2 px-4 rounded text-sm"
					>
						Close
					</button>
				</div>
			</div>
		</div>
	</div>
{/if}