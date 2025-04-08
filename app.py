import streamlit as st

# Open data.json file to read (platform, ok_licenses, bad_licenses) triples into dictionary
import json
import plotly.express as px
import pandas as pd

with open('sorted_license_counter.json') as f:
    data = json.load(f)

df = pd.DataFrame(data)

# Narrow down to 15 most common licenses
df = df.nlargest(15, 'count')

df.head()

st.title("License Distribution")
# Plot DataFrame
fig = px.pie(df, values='count', names='license', title='License Distribution')
# Display the pie chart
st.plotly_chart(fig, use_container_width=True)