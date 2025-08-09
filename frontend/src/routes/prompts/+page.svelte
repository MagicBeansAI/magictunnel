<script lang="ts">
	import { onMount } from 'svelte';
	import { writable } from 'svelte/store';

	interface PromptArgument {
		name: string;
		description?: string;
		required?: boolean;
	}

	interface PromptTemplate {
		name: string;
		description?: string;
		arguments?: PromptArgument[];
	}

	interface PromptMessage {
		role: string;
		content: {
			type: string;
			text: string;
		};
	}

	interface PromptResponse {
		messages: PromptMessage[];
		description?: string;
	}

	let prompts = writable<PromptTemplate[]>([]);
	let selectedPrompt = writable<PromptTemplate | null>(null);
	let promptResponse = writable<PromptResponse | null>(null);
	let loading = false;
	let error = '';
	let searchQuery = '';
	let showExecuteModal = false;
	let showResponseModal = false;
	let executionArguments: Record<string, any> = {};

	// Filter prompts based on search
	$: filteredPrompts = $prompts.filter(prompt => {
		const matchesSearch = !searchQuery || 
			prompt.name.toLowerCase().includes(searchQuery.toLowerCase()) ||
			(prompt.description && prompt.description.toLowerCase().includes(searchQuery.toLowerCase()));
		
		return matchesSearch;
	});

	async function loadPrompts() {
		loading = true;
		error = '';
		try {
			const response = await fetch('/dashboard/api/prompts');
			if (!response.ok) {
				throw new Error(`Failed to load prompts: ${response.status}`);
			}
			const data = await response.json();
			prompts.set(data.prompts || []);
		} catch (err) {
			error = err instanceof Error ? err.message : 'Failed to load prompts';
			console.error('Error loading prompts:', err);
		} finally {
			loading = false;
		}
	}

	function openExecuteModal(prompt: PromptTemplate) {
		selectedPrompt.set(prompt);
		executionArguments = {};
		
		// Initialize arguments with empty values
		if (prompt.arguments) {
			prompt.arguments.forEach(arg => {
				executionArguments[arg.name] = '';
			});
		}
		
		showExecuteModal = true;
	}

	function closeExecuteModal() {
		showExecuteModal = false;
		selectedPrompt.set(null);
		executionArguments = {};
	}

	function closeResponseModal() {
		showResponseModal = false;
		promptResponse.set(null);
	}

	async function executePrompt() {
		if (!$selectedPrompt) return;
		
		loading = true;
		error = '';
		
		try {
			// Filter out empty arguments
			const filteredArgs = Object.entries(executionArguments)
				.filter(([_, value]) => value !== '')
				.reduce((acc, [key, value]) => {
					acc[key] = value;
					return acc;
				}, {} as Record<string, any>);

			const response = await fetch('/dashboard/api/prompts/execute', {
				method: 'POST',
				headers: {
					'Content-Type': 'application/json',
				},
				body: JSON.stringify({
					name: $selectedPrompt.name,
					arguments: Object.keys(filteredArgs).length > 0 ? filteredArgs : undefined
				})
			});
			
			if (!response.ok) {
				throw new Error(`Failed to execute prompt: ${response.status}`);
			}
			
			const data = await response.json();
			promptResponse.set(data.response);
			closeExecuteModal();
			showResponseModal = true;
		} catch (err) {
			error = err instanceof Error ? err.message : 'Failed to execute prompt';
			console.error('Error executing prompt:', err);
		} finally {
			loading = false;
		}
	}

	function getArgumentType(arg: PromptArgument): string {
		// Simple type inference based on name/description
		const name = arg.name.toLowerCase();
		if (name.includes('email')) return 'email';
		if (name.includes('url') || name.includes('link')) return 'url';
		if (name.includes('number') || name.includes('count') || name.includes('age')) return 'number';
		if (name.includes('date')) return 'date';
		return 'text';
	}

	function copyToClipboard(text: string) {
		navigator.clipboard.writeText(text).then(() => {
			// Could add a toast notification here
		}).catch(err => {
			console.error('Failed to copy text: ', err);
		});
	}

	onMount(loadPrompts);
</script>

<svelte:head>
	<title>MCP Prompts - MagicTunnel</title>
</svelte:head>

<div class="min-h-screen bg-gray-50 p-6">
	<div class="max-w-7xl mx-auto">
		<!-- Header -->
		<div class="mb-8">
			<div class="flex justify-between items-center">
				<div>
					<div class="mb-2">
						<h1 class="text-4xl font-bold text-primary-700">MCP Prompt Templates</h1>
					</div>
					<p class="mt-2 text-gray-600">Manage and execute MCP prompt templates with argument substitution</p>
				</div>
				<button
					on:click={loadPrompts}
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

		<!-- Search -->
		<div class="mb-6 bg-white rounded-lg shadow p-4">
			<div>
				<label for="search" class="block text-sm font-medium text-gray-700 mb-2">Search Prompt Templates</label>
				<input
					id="search"
					type="text"
					bind:value={searchQuery}
					placeholder="Search by name or description..."
					class="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500"
				/>
			</div>
		</div>

		<!-- Stats -->
		<div class="mb-6 grid grid-cols-1 md:grid-cols-2 gap-4">
			<div class="bg-white overflow-hidden shadow rounded-lg">
				<div class="p-5">
					<div class="flex items-center">
						<div class="flex-shrink-0">
							<svg class="h-8 w-8 text-purple-500" fill="none" stroke="currentColor" viewBox="0 0 24 24">
								<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 12h.01M12 12h.01M16 12h.01M21 12c0 4.418-4.03 8-9 8a9.863 9.863 0 01-4.255-.949L3 20l1.395-3.72C3.512 15.042 3 13.574 3 12c0-4.418 4.03-8 9-8s9 3.582 9 8z" />
							</svg>
						</div>
						<div class="ml-5 w-0 flex-1">
							<dl>
								<dt class="text-sm font-medium text-gray-500 truncate">Total Templates</dt>
								<dd class="text-lg font-medium text-gray-900">{$prompts.length}</dd>
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
								<dd class="text-lg font-medium text-gray-900">{filteredPrompts.length}</dd>
							</dl>
						</div>
					</div>
				</div>
			</div>
		</div>

		<!-- Prompts List -->
		<div class="bg-white shadow overflow-hidden rounded-md">
			<div class="px-4 py-5 sm:p-6">
				<h3 class="text-lg leading-6 font-medium text-gray-900 mb-4">Available Prompt Templates</h3>
				
				{#if loading}
					<div class="text-center py-8">
						<svg class="animate-spin mx-auto h-8 w-8 text-blue-500" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24">
							<circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
							<path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
						</svg>
						<p class="mt-2 text-gray-500">Loading prompt templates...</p>
					</div>
				{:else if filteredPrompts.length === 0}
					<div class="text-center py-8">
						<svg class="mx-auto h-12 w-12 text-gray-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
							<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 12h.01M12 12h.01M16 12h.01M21 12c0 4.418-4.03 8-9 8a9.863 9.863 0 01-4.255-.949L3 20l1.395-3.72C3.512 15.042 3 13.574 3 12c0-4.418 4.03-8 9-8s9 3.582 9 8z" />
						</svg>
						<h3 class="mt-2 text-sm font-medium text-gray-900">No prompt templates found</h3>
						<p class="mt-1 text-sm text-gray-500">
							{searchQuery ? 'Try adjusting your search.' : 'No MCP prompt templates are currently available.'}
						</p>
					</div>
				{:else}
					<div class="space-y-4">
						{#each filteredPrompts as prompt}
							<div class="border border-gray-200 rounded-lg p-4 hover:bg-gray-50 transition-colors">
								<div class="flex justify-between items-start">
									<div class="flex-1">
										<h4 class="text-lg font-medium text-gray-900">{prompt.name}</h4>
										{#if prompt.description}
											<p class="text-sm text-gray-700 mt-2">{prompt.description}</p>
										{/if}
										{#if prompt.arguments && prompt.arguments.length > 0}
											<div class="mt-3">
												<h5 class="text-sm font-medium text-gray-700 mb-2">Arguments:</h5>
												<div class="space-y-1">
													{#each prompt.arguments as arg}
														<div class="flex items-center space-x-2">
															<span class="inline-flex items-center px-2 py-0.5 rounded text-xs font-medium {arg.required ? 'bg-red-100 text-red-800' : 'bg-gray-100 text-gray-800'}">
																{arg.name}
															</span>
															{#if arg.required}
																<span class="text-xs text-red-600">required</span>
															{/if}
															{#if arg.description}
																<span class="text-xs text-gray-500">- {arg.description}</span>
															{/if}
														</div>
													{/each}
												</div>
											</div>
										{/if}
									</div>
									<div class="ml-4">
										<button
											on:click={() => openExecuteModal(prompt)}
											disabled={loading}
											class="bg-purple-500 hover:bg-purple-700 disabled:bg-gray-400 text-white font-bold py-2 px-4 rounded text-sm"
										>
											Execute Template
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

<!-- Execute Prompt Modal -->
{#if showExecuteModal && $selectedPrompt}
	<div class="fixed inset-0 bg-gray-600 bg-opacity-50 overflow-y-auto h-full w-full z-50">
		<div class="relative top-20 mx-auto p-5 border w-11/12 md:w-3/4 lg:w-1/2 shadow-lg rounded-md bg-white">
			<div class="mt-3">
				<!-- Modal Header -->
				<div class="flex justify-between items-center pb-3 border-b">
					<h3 class="text-lg font-medium text-gray-900">Execute Prompt Template</h3>
					<button
						on:click={closeExecuteModal}
						class="text-gray-400 hover:text-gray-600"
					>
						<svg class="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
							<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
						</svg>
					</button>
				</div>

				<!-- Template Info -->
				<div class="mt-4">
					<div class="bg-gray-50 p-3 rounded-lg">
						<p class="text-sm"><strong>Template:</strong> {$selectedPrompt.name}</p>
						{#if $selectedPrompt.description}
							<p class="text-sm mt-1"><strong>Description:</strong> {$selectedPrompt.description}</p>
						{/if}
					</div>
				</div>

				<!-- Arguments Form -->
				{#if $selectedPrompt.arguments && $selectedPrompt.arguments.length > 0}
					<div class="mt-4">
						<h4 class="text-md font-medium text-gray-900 mb-3">Template Arguments</h4>
						<div class="space-y-4">
							{#each $selectedPrompt.arguments as arg}
								<div>
									<label for={arg.name} class="block text-sm font-medium text-gray-700 mb-1">
										{arg.name}
										{#if arg.required}
											<span class="text-red-500">*</span>
										{/if}
									</label>
									{#if arg.description}
										<p class="text-xs text-gray-500 mb-2">{arg.description}</p>
									{/if}
									<input
										id={arg.name}
										type={getArgumentType(arg)}
										bind:value={executionArguments[arg.name]}
										placeholder="Enter {arg.name}..."
										required={arg.required}
										class="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-purple-500"
									/>
								</div>
							{/each}
						</div>
					</div>
				{:else}
					<div class="mt-4 text-center text-gray-500">
						<p>This template has no arguments to configure.</p>
					</div>
				{/if}

				<!-- Modal Actions -->
				<div class="flex justify-end space-x-3 mt-6 pt-3 border-t">
					<button
						on:click={closeExecuteModal}
						class="bg-gray-500 hover:bg-gray-700 text-white font-bold py-2 px-4 rounded text-sm"
					>
						Cancel
					</button>
					<button
						on:click={executePrompt}
						disabled={loading}
						class="bg-purple-500 hover:bg-purple-700 disabled:bg-gray-400 text-white font-bold py-2 px-4 rounded text-sm inline-flex items-center"
					>
						{#if loading}
							<svg class="animate-spin -ml-1 mr-2 h-4 w-4 text-white" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24">
								<circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
								<path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
							</svg>
							Executing...
						{:else}
							Execute Template
						{/if}
					</button>
				</div>
			</div>
		</div>
	</div>
{/if}

<!-- Prompt Response Modal -->
{#if showResponseModal && $promptResponse}
	<div class="fixed inset-0 bg-gray-600 bg-opacity-50 overflow-y-auto h-full w-full z-50">
		<div class="relative top-10 mx-auto p-5 border w-11/12 md:w-4/5 lg:w-3/4 shadow-lg rounded-md bg-white">
			<div class="mt-3">
				<!-- Modal Header -->
				<div class="flex justify-between items-center pb-3 border-b">
					<h3 class="text-lg font-medium text-gray-900">Prompt Response</h3>
					<button
						on:click={closeResponseModal}
						class="text-gray-400 hover:text-gray-600"
					>
						<svg class="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
							<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
						</svg>
					</button>
				</div>

				<!-- Response Description -->
				{#if $promptResponse.description}
					<div class="mt-4">
						<div class="bg-blue-50 p-3 rounded-lg">
							<p class="text-sm text-blue-800">{$promptResponse.description}</p>
						</div>
					</div>
				{/if}

				<!-- Messages -->
				<div class="mt-4">
					<h4 class="text-md font-medium text-gray-900 mb-3">Generated Messages</h4>
					<div class="space-y-4 max-h-96 overflow-y-auto">
						{#each $promptResponse.messages as message, index}
							<div class="border border-gray-200 rounded-lg p-4">
								<div class="flex justify-between items-start mb-2">
									<div class="flex items-center space-x-2">
										<span class="inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium bg-purple-100 text-purple-800">
											{message.role}
										</span>
										<span class="text-xs text-gray-500">Message {index + 1}</span>
									</div>
									<button
										on:click={() => copyToClipboard(message.content.text)}
										class="text-gray-400 hover:text-gray-600 p-1"
										title="Copy to clipboard"
									>
										<svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
											<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 16H6a2 2 0 01-2-2V6a2 2 0 012-2h8a2 2 0 012 2v2m-6 12h8a2 2 0 002-2v-8a2 2 0 00-2-2h-8a2 2 0 00-2 2v8a2 2 0 002 2z" />
										</svg>
									</button>
								</div>
								<div class="bg-gray-50 p-3 rounded">
									<pre class="text-sm whitespace-pre-wrap font-mono">{message.content.text}</pre>
								</div>
							</div>
						{/each}
					</div>
				</div>

				<!-- Modal Actions -->
				<div class="flex justify-end space-x-3 mt-6 pt-3 border-t">
					<button
						on:click={closeResponseModal}
						class="bg-gray-500 hover:bg-gray-700 text-white font-bold py-2 px-4 rounded text-sm"
					>
						Close
					</button>
				</div>
			</div>
		</div>
	</div>
{/if}