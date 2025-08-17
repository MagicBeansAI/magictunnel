<script lang="ts">
	import { onMount } from 'svelte';
	import { writable } from 'svelte/store';
	import { page } from '$app/stores';
	import RuleManagement from './components/RuleManagement.svelte';
	import EmergencyControls from './components/EmergencyControls.svelte';
	import AuditLogViewer from './components/AuditLogViewer.svelte';
	import PatternTesting from './components/PatternTesting.svelte';
	import ChangeTracking from './components/ChangeTracking.svelte';
	import SecurityMetrics from './components/SecurityMetrics.svelte';

	// Stores for component state management
	export const securityState = writable({
		emergencyLockdown: {
			isActive: false,
			activatedAt: null,
			activatedBy: null,
			blockedRequests: 0
		},
		ruleStats: {
			totalRules: 0,
			activeRules: 0,
			conflicts: 0
		},
		auditStats: {
			totalEvents: 0,
			recentViolations: 0,
			recentChanges: 0
		}
	});

	let activeTab = 'rules';
	let loading = true;
	let error = null;

	// Tab configuration
	const tabs = [
		{ id: 'rules', label: 'Allowlist Rules', icon: '🔧' },
		{ id: 'emergency', label: 'Emergency Controls', icon: '🚨' },
		{ id: 'audit', label: 'Audit Logs', icon: '📋' },
		{ id: 'patterns', label: 'Pattern Testing', icon: '🧪' },
		{ id: 'changes', label: 'Change Tracking', icon: '📊' },
		{ id: 'metrics', label: 'Security Metrics', icon: '📈' }
	];

	onMount(async () => {
		try {
			await loadSecurityOverview();
			
			// Check for tab parameter in URL
			const urlParams = new URLSearchParams($page.url.search);
			const tabParam = urlParams.get('tab');
			if (tabParam && tabs.some(tab => tab.id === tabParam)) {
				activeTab = tabParam;
			}
		} catch (err) {
			error = err.message;
		} finally {
			loading = false;
		}
	});

	async function loadSecurityOverview() {
		try {
			// Load emergency lockdown status
			const emergencyResponse = await fetch('/api/security/emergency/lockdown/status');
			if (emergencyResponse.ok) {
				const emergencyData = await emergencyResponse.json();
				securityState.update(state => ({
					...state,
					emergencyLockdown: emergencyData?.data || {
						isActive: false,
						activatedAt: null,
						activatedBy: null,
						blockedRequests: 0
					}
				}));
			}

			// Load unified rules stats
			const rulesResponse = await fetch('/api/security/rules/unified?format=json');
			if (rulesResponse.ok) {
				const rulesData = await rulesResponse.json();
				const ruleStats = rulesData?.data?.statistics || {};
				securityState.update(state => ({
					...state,
					ruleStats: {
						totalRules: ruleStats.total_rules || 0,
						activeRules: ruleStats.total_rules || 0,
						conflicts: ruleStats.conflicts || 0
					}
				}));
			}

			// Load audit statistics
			const auditResponse = await fetch('/api/security/audit/statistics');
			if (auditResponse.ok) {
				const auditData = await auditResponse.json();
				const auditStats = auditData?.data || {};
				securityState.update(state => ({
					...state,
					auditStats: {
						totalEvents: auditStats.total_events || 0,
						recentViolations: auditStats.recent_violations || 0,
						recentChanges: auditStats.recent_changes || 0
					}
				}));
			}

			// Load change tracking statistics
			const changesResponse = await fetch('/api/security/changes/statistics');
			if (changesResponse.ok) {
				const changesData = await changesResponse.json();
				const changeStats = changesData?.data || {};
				securityState.update(state => ({
					...state,
					auditStats: {
						...state.auditStats,
						recentChanges: changeStats.recent_changes || 0
					}
				}));
			}
		} catch (err) {
			console.error('Failed to load security overview:', err);
			throw err;
		}
	}

	function setActiveTab(tabId: string) {
		activeTab = tabId;
	}

	// Auto-refresh security state every 30 seconds
	let refreshInterval;
	onMount(() => {
		refreshInterval = setInterval(async () => {
			try {
				await loadSecurityOverview();
			} catch (err) {
				console.warn('Failed to refresh security overview:', err);
			}
		}, 30000);

		return () => {
			if (refreshInterval) {
				clearInterval(refreshInterval);
			}
		};
	});
</script>

<svelte:head>
	<title>Security Management - MagicTunnel</title>
</svelte:head>

<div class="security-management">
	<div class="header">
		<h1>🔐 Security Management Dashboard</h1>
		<p>Comprehensive security rule management, monitoring, and control center</p>
		
		{#if $securityState.emergencyLockdown.isActive}
			<div class="emergency-banner">
				🚨 <strong>EMERGENCY LOCKDOWN ACTIVE</strong> - All tool requests are being blocked
				<span class="lockdown-details">
					Activated by: {$securityState.emergencyLockdown.activatedBy || 'Unknown'} | 
					Blocked Requests: {$securityState.emergencyLockdown.blockedRequests}
				</span>
			</div>
		{/if}
	</div>

	{#if loading}
		<div class="loading">
			<div class="spinner"></div>
			<p>Loading security management dashboard...</p>
		</div>
	{:else if error}
		<div class="error">
			<h3>⚠️ Error Loading Security Dashboard</h3>
			<p>{error}</p>
			<button on:click={() => window.location.reload()}>Retry</button>
		</div>
	{:else}
		<!-- Security Overview Cards -->
		<div class="overview-cards">
			<div class="card {$securityState.emergencyLockdown.isActive ? 'emergency' : 'normal'}">
				<h3>🚨 Emergency Status</h3>
				<div class="card-value">
					{$securityState.emergencyLockdown.isActive ? 'LOCKDOWN ACTIVE' : 'Normal Operation'}
				</div>
				<div class="card-detail">
					{#if $securityState.emergencyLockdown.isActive}
						{$securityState.emergencyLockdown.blockedRequests} requests blocked
					{:else}
						System operational
					{/if}
				</div>
			</div>

			<div class="card">
				<h3>🔧 Security Rules</h3>
				<div class="card-value">{$securityState.ruleStats.totalRules}</div>
				<div class="card-detail">
					{$securityState.ruleStats.activeRules} active
					{#if $securityState.ruleStats.conflicts > 0}
						• <span class="warning">{$securityState.ruleStats.conflicts} conflicts</span>
					{/if}
				</div>
			</div>

			<div class="card">
				<h3>📋 Audit Events</h3>
				<div class="card-value">{$securityState.auditStats.totalEvents}</div>
				<div class="card-detail">
					{$securityState.auditStats.recentViolations} recent violations
				</div>
			</div>

			<div class="card">
				<h3>📊 Recent Changes</h3>
				<div class="card-value">{$securityState.auditStats.recentChanges}</div>
				<div class="card-detail">Configuration changes (24h)</div>
			</div>
		</div>

		<!-- Tab Navigation -->
		<div class="tab-navigation">
			{#each tabs as tab}
				<button 
					class="tab {activeTab === tab.id ? 'active' : ''}"
					on:click={() => setActiveTab(tab.id)}
				>
					<span class="tab-icon">{tab.icon}</span>
					<span class="tab-label">{tab.label}</span>
				</button>
			{/each}
		</div>

		<!-- Tab Content -->
		<div class="tab-content">
			{#if activeTab === 'rules'}
				<RuleManagement {securityState} on:refresh={loadSecurityOverview} />
			{:else if activeTab === 'emergency'}
				<EmergencyControls {securityState} on:refresh={loadSecurityOverview} />
			{:else if activeTab === 'audit'}
				<AuditLogViewer {securityState} on:refresh={loadSecurityOverview} />
			{:else if activeTab === 'patterns'}
				<PatternTesting {securityState} on:refresh={loadSecurityOverview} />
			{:else if activeTab === 'changes'}
				<ChangeTracking {securityState} on:refresh={loadSecurityOverview} />
			{:else if activeTab === 'metrics'}
				<SecurityMetrics {securityState} on:refresh={loadSecurityOverview} />
			{/if}
		</div>
	{/if}
</div>

<style>
	.security-management {
		padding: 20px;
		max-width: 1400px;
		margin: 0 auto;
	}

	.header {
		margin-bottom: 30px;
		text-align: center;
	}

	.header h1 {
		color: #2563eb;
		margin-bottom: 8px;
		font-size: 2.5em;
	}

	.header p {
		color: #6b7280;
		font-size: 1.1em;
	}

	.emergency-banner {
		background: linear-gradient(135deg, #ef4444, #dc2626);
		color: white;
		padding: 15px 20px;
		border-radius: 8px;
		margin-top: 20px;
		text-align: center;
		box-shadow: 0 4px 6px rgba(239, 68, 68, 0.3);
		animation: pulse 2s infinite;
	}

	.lockdown-details {
		display: block;
		margin-top: 5px;
		font-size: 0.9em;
		opacity: 0.9;
	}

	@keyframes pulse {
		0%, 100% { transform: scale(1); }
		50% { transform: scale(1.02); }
	}

	.loading {
		text-align: center;
		padding: 60px 20px;
	}

	.spinner {
		width: 40px;
		height: 40px;
		border: 4px solid #f3f4f6;
		border-top: 4px solid #2563eb;
		border-radius: 50%;
		animation: spin 1s linear infinite;
		margin: 0 auto 20px;
	}

	@keyframes spin {
		0% { transform: rotate(0deg); }
		100% { transform: rotate(360deg); }
	}

	.error {
		text-align: center;
		padding: 40px 20px;
		background: #fef2f2;
		border: 1px solid #fecaca;
		border-radius: 8px;
		color: #dc2626;
	}

	.error button {
		background: #dc2626;
		color: white;
		border: none;
		padding: 10px 20px;
		border-radius: 6px;
		cursor: pointer;
		margin-top: 15px;
	}

	.overview-cards {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(250px, 1fr));
		gap: 20px;
		margin-bottom: 30px;
	}

	.card {
		background: white;
		padding: 24px;
		border-radius: 12px;
		box-shadow: 0 2px 8px rgba(0, 0, 0, 0.1);
		border: 1px solid #e5e7eb;
		transition: transform 0.2s, box-shadow 0.2s;
	}

	.card:hover {
		transform: translateY(-2px);
		box-shadow: 0 4px 12px rgba(0, 0, 0, 0.15);
	}

	.card.emergency {
		border-color: #ef4444;
		background: linear-gradient(135deg, #fef2f2, #ffffff);
	}

	.card h3 {
		margin: 0 0 10px 0;
		font-size: 1em;
		color: #6b7280;
		font-weight: 600;
	}

	.card-value {
		font-size: 2em;
		font-weight: bold;
		color: #111827;
		margin-bottom: 5px;
	}

	.card.emergency .card-value {
		color: #dc2626;
	}

	.card-detail {
		color: #6b7280;
		font-size: 0.9em;
	}

	.warning {
		color: #f59e0b;
		font-weight: 600;
	}

	.tab-navigation {
		display: flex;
		flex-wrap: wrap;
		gap: 5px;
		margin-bottom: 30px;
		border-bottom: 2px solid #e5e7eb;
		padding-bottom: 0;
	}

	.tab {
		display: flex;
		align-items: center;
		gap: 8px;
		padding: 12px 20px;
		background: none;
		border: none;
		border-bottom: 3px solid transparent;
		cursor: pointer;
		transition: all 0.2s;
		color: #6b7280;
		font-weight: 500;
	}

	.tab:hover {
		background: #f9fafb;
		color: #374151;
	}

	.tab.active {
		color: #2563eb;
		border-bottom-color: #2563eb;
		background: #eff6ff;
	}

	.tab-icon {
		font-size: 1.2em;
	}

	.tab-content {
		background: white;
		border-radius: 12px;
		border: 1px solid #e5e7eb;
		box-shadow: 0 2px 8px rgba(0, 0, 0, 0.1);
		min-height: 600px;
	}

	/* Responsive design */
	@media (max-width: 768px) {
		.security-management {
			padding: 15px;
		}

		.header h1 {
			font-size: 2em;
		}

		.overview-cards {
			grid-template-columns: 1fr;
		}

		.tab-navigation {
			flex-direction: column;
		}

		.tab {
			justify-content: center;
		}
	}
</style>