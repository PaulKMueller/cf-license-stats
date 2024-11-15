import streamlit as st

# Open data.json file to read (platform, ok_licenses, bad_licenses) triples into dictionary
import json
import plotly.express as px

with open('data.json') as f:
    data = json.load(f)

# Streamlit app title
st.title("Platform License Distribution")

# Iterate over each platform and create pie charts
for platform, ok_licenses, bad_licenses in data:
    # Data preparation for Plotly
    chart_data = {
        'License Type': ['OK Licenses', 'Bad Licenses'],
        'Count': [ok_licenses, bad_licenses]
    }

    # Create a pie chart with Plotly Express
    fig = px.pie(
        chart_data,
        names='License Type',
        values='Count',
        title=f"License Distribution for {platform}",
        hole=0.3  # Optional, creates a donut chart if > 0
    )

    # Display the chart in Streamlit
    st.plotly_chart(fig, use_container_width=True)